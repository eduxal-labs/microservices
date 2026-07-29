#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request


def cf_api_request(method, endpoint, token, payload=None):
    url = f"https://api.cloudflare.com/client/v4/{endpoint}"
    data = json.dumps(payload).encode("utf-8") if payload else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        print(f"Cloudflare API Error: {e.read().decode('utf-8')}")
        sys.exit(1)


def upsert_cloudflare_cname(zone_id, token, name, target, proxied=False):
    clean_name = name.rstrip(".")
    clean_target = target.rstrip(".")

    records = cf_api_request(
        "GET", f"zones/{zone_id}/dns_records?name={clean_name}&type=CNAME", token
    )
    results = records.get("result", [])

    payload = {
        "type": "CNAME",
        "name": clean_name,
        "content": clean_target,
        "ttl": 1 if proxied else 120,
        "proxied": proxied,
    }

    if results:
        record_id = results[0]["id"]
        print(f"Updating Cloudflare CNAME: {clean_name} -> {clean_target}")
        cf_api_request("PUT", f"zones/{zone_id}/dns_records/{record_id}", token, payload)
    else:
        print(f"Creating Cloudflare CNAME: {clean_name} -> {clean_target}")
        cf_api_request("POST", f"zones/{zone_id}/dns_records", token, payload)


def ensure_acm_certificate(domain_name, region, zone_id, cf_token):
    print(f"=== Ensuring ACM Certificate for {domain_name} in {region} ===")
    
    # 1. List certificates to see if one exists
    cmd = ["aws", "acm", "list-certificates", "--region", region, "--output", "json"]
    res = subprocess.run(cmd, capture_output=True, text=True, check=True)
    certs = json.loads(res.stdout).get("CertificateSummaryList", [])

    cert_arn = None
    for cert in certs:
        if cert.get("DomainName") == domain_name:
            cert_arn = cert.get("CertificateArn")
            print(f"Found existing ACM Certificate: {cert_arn}")
            break

    if not cert_arn:
        print(f"Requesting new ACM Certificate for {domain_name}...")
        req_cmd = [
            "aws",
            "acm",
            "request-certificate",
            "--domain-name",
            domain_name,
            "--validation-method",
            "DNS",
            "--region",
            region,
            "--output",
            "json",
        ]
        req_res = subprocess.run(req_cmd, capture_output=True, text=True, check=True)
        cert_arn = json.loads(req_res.stdout).get("CertificateArn")
        print(f"Requested new certificate: {cert_arn}")

    # 2. Get DNS Validation Record
    cname_name, cname_value = None, None
    for _ in range(30):
        desc_cmd = [
            "aws",
            "acm",
            "describe-certificate",
            "--certificate-arn",
            cert_arn,
            "--region",
            region,
            "--output",
            "json",
        ]
        desc_res = subprocess.run(desc_cmd, capture_output=True, text=True, check=True)
        cert_detail = json.loads(desc_res.stdout).get("Certificate", {})
        
        status = cert_detail.get("Status")
        if status == "ISSUED":
            print(f"ACM Certificate is ISSUED: {cert_arn}")
            return cert_arn

        domain_validations = cert_detail.get("DomainValidationOptions", [])
        for opt in domain_validations:
            if opt.get("DomainName") == domain_name and "ResourceRecord" in opt:
                rec = opt["ResourceRecord"]
                cname_name, cname_value = rec["Name"], rec["Value"]
                break
        
        if cname_name and cname_value:
            break
            
        print("Waiting for ACM validation record generation...")
        time.sleep(3)

    if cname_name and cname_value:
        print(f"Upserting ACM DNS validation CNAME to Cloudflare: {cname_name} -> {cname_value}")
        upsert_cloudflare_cname(zone_id, cf_token, cname_name, cname_value, proxied=False)

    # 3. Wait for Certificate to become ISSUED
    print("Waiting for ACM Certificate DNS validation...")
    for i in range(60):
        desc_cmd = [
            "aws",
            "acm",
            "describe-certificate",
            "--certificate-arn",
            cert_arn,
            "--region",
            region,
            "--output",
            "json",
        ]
        desc_res = subprocess.run(desc_cmd, capture_output=True, text=True, check=True)
        status = json.loads(desc_res.stdout).get("Certificate", {}).get("Status")
        if status == "ISSUED":
            print(f"ACM Certificate successfully ISSUED: {cert_arn}")
            return cert_arn
        print(f"[{i+1}/60] ACM Certificate status: {status}... sleeping 5s")
        time.sleep(5)

    print("Timed out waiting for ACM certificate validation")
    sys.exit(1)


def main():
    token = os.environ.get("CLOUDFLARE_API_TOKEN")
    zone_id = os.environ.get("CLOUDFLARE_ZONE_ID")
    domain_name = os.environ.get("DOMAIN_NAME", "auth.eduxal.com")
    region = os.environ.get("AWS_REGION", "us-east-1")

    if not token or not zone_id:
        print("Error: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ZONE_ID must be set")
        sys.exit(1)

    cert_arn = ensure_acm_certificate(domain_name, region, zone_id, token)
    
    # Write cert_arn to environment for SAM
    if "GITHUB_OUTPUT" in os.environ:
        with open(os.environ["GITHUB_OUTPUT"], "a") as f:
            f.write(f"cert_arn={cert_arn}\n")


if __name__ == "__main__":
    main()
