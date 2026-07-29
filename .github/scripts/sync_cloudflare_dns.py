#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request


def run_aws_cmd(cmd):
    res = subprocess.run(cmd, capture_output=True, text=True)
    return res.stdout, res.stderr, res.returncode


def get_stack_output(stack_name, output_key, region):
    stdout, stderr, code = run_aws_cmd([
        "aws", "cloudformation", "describe-stacks",
        "--stack-name", stack_name,
        "--region", region,
        "--output", "json"
    ])
    if code != 0:
        print(f"Error describing stack {stack_name}: {stderr}")
        sys.exit(1)
    
    data = json.loads(stdout)
    outputs = data.get("Stacks", [])[0].get("Outputs", [])
    for out in outputs:
        if out.get("OutputKey") == output_key:
            return out.get("OutputValue")
    print(f"Error: Stack output key {output_key} not found")
    sys.exit(1)


def debug_domain_names(region):
    print("=== Listing V1 Domain Names ===")
    out_v1, err_v1, _ = run_aws_cmd(["aws", "apigateway", "get-domain-names", "--region", region])
    print(f"V1: {out_v1}")

    print("=== Listing V2 Domain Names ===")
    out_v2, err_v2, _ = run_aws_cmd(["aws", "apigatewayv2", "get-domain-names", "--region", region])
    print(f"V2: {out_v2}")


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
        err_msg = e.read().decode('utf-8')
        print(f"Cloudflare API Error ({endpoint}): {err_msg}")
        return None


def upsert_cloudflare_cname(zone_id, token, name, target):
    clean_name = name.rstrip(".")
    clean_target = target.rstrip(".")

    records = cf_api_request(
        "GET", f"zones/{zone_id}/dns_records?name={clean_name}&type=CNAME", token
    ) or {}
    results = records.get("result", [])

    payload = {
        "type": "CNAME",
        "name": clean_name,
        "content": clean_target,
        "ttl": 1,
        "proxied": True,
    }

    if results:
        record_id = results[0]["id"]
        print(f"Updating existing Cloudflare CNAME: {clean_name} -> {clean_target}")
        cf_api_request(
            "PUT", f"zones/{zone_id}/dns_records/{record_id}", token, payload
        )
    else:
        print(f"Creating new Cloudflare CNAME: {clean_name} -> {clean_target}")
        cf_api_request("POST", f"zones/{zone_id}/dns_records", token, payload)


def main():
    token = os.environ.get("CLOUDFLARE_API_TOKEN")
    zone_id = os.environ.get("CLOUDFLARE_ZONE_ID")
    domain_name = os.environ.get("DOMAIN_NAME", "auth.eduxal.com")
    region = os.environ.get("AWS_REGION", "us-east-1")
    cert_arn = os.environ.get("CERTIFICATE_ARN")
    stack_name = "eduxal-microservices"

    if not token or not zone_id:
        print("Error: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ZONE_ID must be set")
        sys.exit(1)

    print(f"=== Syncing Domain Mapping and Cloudflare DNS for {domain_name} ===")
    
    # Debug existing domain registrations
    debug_domain_names(region)

    # Fetch HttpApiId from deployed CloudFormation stack
    http_api_id = get_stack_output(stack_name, "HttpApiId", region)
    target_host = f"{http_api_id}.execute-api.{region}.amazonaws.com"
    print(f"Target AWS HTTP API Domain: {target_host}")

    # Update Cloudflare DNS CNAME record
    upsert_cloudflare_cname(zone_id, token, domain_name, target_host)

    print("=== Domain & Cloudflare DNS Sync Complete! ===")


if __name__ == "__main__":
    main()
