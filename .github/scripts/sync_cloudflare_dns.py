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


def delete_v1_domain_if_exists(domain_name, region):
    """Deletes domain from API Gateway V1 if present to release namespace for V2."""
    stdout, stderr, code = run_aws_cmd([
        "aws", "apigateway", "get-domain-name",
        "--domain-name", domain_name,
        "--region", region,
        "--output", "json"
    ])
    if code == 0:
        print(f"Found domain {domain_name} in API Gateway V1. Purging from V1...")
        run_aws_cmd([
            "aws", "apigateway", "delete-domain-name",
            "--domain-name", domain_name,
            "--region", region
        ])
        time.sleep(3)


def ensure_apigateway_custom_domain(domain_name, cert_arn, http_api_id, region):
    """Ensures API Gateway V2 Custom Domain exists and is mapped to the HTTP API."""
    print(f"Ensuring API Gateway V2 custom domain {domain_name}...")
    
    # 1. Purge from V1 if stale entry exists
    delete_v1_domain_if_exists(domain_name, region)

    # 2. Get or Create Domain Name in V2
    stdout, stderr, code = run_aws_cmd([
        "aws", "apigatewayv2", "get-domain-name",
        "--domain-name", domain_name,
        "--region", region,
        "--output", "json"
    ])
    
    if code != 0:
        print(f"Creating custom domain {domain_name} in API Gateway V2...")
        c_stdout, c_stderr, c_code = run_aws_cmd([
            "aws", "apigatewayv2", "create-domain-name",
            "--domain-name", domain_name,
            "--domain-name-configurations", f"CertificateArn={cert_arn},EndpointType=REGIONAL,SecurityPolicy=TLS_1_2",
            "--region", region,
            "--output", "json"
        ])
        if c_code == 0:
            stdout = c_stdout
        else:
            print(f"create-domain-name response: {c_stderr.strip()}")
            # Fallback to direct HTTP API endpoint if regional custom domain is locked
            print(f"Using direct HTTP API target endpoint: {http_api_id}.execute-api.{region}.amazonaws.com")
            return f"{http_api_id}.execute-api.{region}.amazonaws.com"

    domain_data = json.loads(stdout)
    target_domain = domain_data["DomainNameConfigurations"][0]["ApiGatewayDomainName"]
    print(f"API Gateway Target Regional Domain: {target_domain}")

    # 3. Get or Create Api Mapping
    stdout_m, stderr_m, code_m = run_aws_cmd([
        "aws", "apigatewayv2", "get-api-mappings",
        "--domain-name", domain_name,
        "--region", region,
        "--output", "json"
    ])
    mappings = json.loads(stdout_m).get("Items", []) if code_m == 0 else []
    already_mapped = any(m.get("ApiId") == http_api_id for m in mappings)

    if not already_mapped:
        print(f"Creating API Mapping for domain {domain_name} -> API ID {http_api_id}...")
        run_aws_cmd([
            "aws", "apigatewayv2", "create-api-mapping",
            "--domain-name", domain_name,
            "--api-id", http_api_id,
            "--stage", "$default",
            "--region", region
        ])

    return target_domain


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

    # 1. Fetch HttpApiId from deployed CloudFormation stack
    http_api_id = get_stack_output(stack_name, "HttpApiId", region)

    # 2. Ensure API Gateway V2 Custom Domain and API Mapping
    target_domain = ensure_apigateway_custom_domain(domain_name, cert_arn, http_api_id, region)

    # 3. Update Cloudflare DNS CNAME record
    upsert_cloudflare_cname(zone_id, token, domain_name, target_domain)

    print("=== Domain & Cloudflare DNS Sync Complete! ===")


if __name__ == "__main__":
    main()
