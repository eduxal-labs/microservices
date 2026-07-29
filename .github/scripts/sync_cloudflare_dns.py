#!/usr/bin/env python3
import json
import os
import subprocess
import sys
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


def upsert_cloudflare_host_rewrite(zone_id, token, domain_name, target_host):
    """Sets up a Cloudflare Transform Rule to override the Host header to target_host."""
    print(f"Setting up Cloudflare Host header rewrite: {domain_name} -> {target_host}...")
    
    rule = {
        "action": "rewrite",
        "action_parameters": {
            "headers": {
                "Host": {
                    "operation": "set",
                    "value": target_host
                }
            }
        },
        "expression": f'(http.host eq "{domain_name}")',
        "description": f"Rewrite Host header for {domain_name} to AWS API Gateway"
    }

    # Fetch entrypoint ruleset for http_request_late_transform
    rulesets = cf_api_request("GET", f"zones/{zone_id}/rulesets", token) or {}
    items = rulesets.get("result", [])
    entrypoint = next((r for r in items if r.get("phase") == "http_request_late_transform"), None)

    if entrypoint:
        ruleset_id = entrypoint["id"]
        print(f"Updating existing ruleset {ruleset_id}...")
        cf_api_request("PUT", f"zones/{zone_id}/rulesets/{ruleset_id}", token, {
            "rules": [rule]
        })
    else:
        print("Creating new http_request_late_transform ruleset...")
        cf_api_request("POST", f"zones/{zone_id}/rulesets", token, {
            "name": "API Gateway Host Rewrite",
            "kind": "zone",
            "phase": "http_request_late_transform",
            "rules": [rule]
        })


def main():
    token = os.environ.get("CLOUDFLARE_API_TOKEN")
    zone_id = os.environ.get("CLOUDFLARE_ZONE_ID")
    domain_name = os.environ.get("DOMAIN_NAME", "auth.eduxal.com")
    region = os.environ.get("AWS_REGION", "us-east-1")
    stack_name = "eduxal-microservices"

    if not token or not zone_id:
        print("Error: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ZONE_ID must be set")
        sys.exit(1)

    print(f"=== Syncing Cloudflare DNS and Host Header Rewrite for {domain_name} ===")

    # 1. Fetch HttpApiId from deployed CloudFormation stack
    http_api_id = get_stack_output(stack_name, "HttpApiId", region)
    target_host = f"{http_api_id}.execute-api.{region}.amazonaws.com"
    print(f"Target AWS HTTP API Domain: {target_host}")

    # 2. Update Cloudflare DNS CNAME record
    upsert_cloudflare_cname(zone_id, token, domain_name, target_host)

    # 3. Upsert Cloudflare Host Header Transform Rule
    upsert_cloudflare_host_rewrite(zone_id, token, domain_name, target_host)

    print("=== Domain & Cloudflare DNS Sync Complete! ===")


if __name__ == "__main__":
    main()
