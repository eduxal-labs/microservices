#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request


def get_apigateway_domain_target(domain_name, region):
    """Retrieves API Gateway regional target domain name using AWS CLI."""
    cmd = [
        "aws",
        "apigatewayv2",
        "get-domain-name",
        "--domain-name",
        domain_name,
        "--region",
        region,
        "--output",
        "json",
    ]
    res = subprocess.run(cmd, capture_output=True, text=True, check=True)
    domain_data = json.loads(res.stdout)
    configs = domain_data.get("DomainNameConfigurations", [])
    if configs:
        return configs[0].get("ApiGatewayDomainName")
    print(f"Error: Target domain name not found for {domain_name}")
    sys.exit(1)


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
    # Remove trailing dot if present
    clean_name = name.rstrip(".")
    clean_target = target.rstrip(".")

    # Check if record exists
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

    if not token or not zone_id:
        print(
            "Error: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ZONE_ID environment variables must be set"
        )
        sys.exit(1)

    print(f"=== Syncing Cloudflare DNS for {domain_name} ===")

    # Retrieve API Gateway regional target domain and update Cloudflare CNAME
    target_domain = get_apigateway_domain_target(domain_name, region)
    print(f"API Gateway Target: {domain_name} -> {target_domain}")
    upsert_cloudflare_cname(zone_id, token, domain_name, target_domain, proxied=True)

    print("=== Cloudflare DNS Sync Complete! ===")


if __name__ == "__main__":
    main()
