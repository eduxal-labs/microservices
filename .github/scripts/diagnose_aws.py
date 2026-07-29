#!/usr/bin/env python3
import json
import subprocess
import sys

def run_cmd(cmd):
    res = subprocess.run(cmd, capture_output=True, text=True)
    return res.stdout, res.stderr, res.returncode

def main():
    print("=== AWS DIAGNOSTICS FOR auth.eduxal.com ===")
    
    print("\n1. Checking API Gateway V2 Domain Names:")
    stdout, stderr, code = run_cmd(["aws", "apigatewayv2", "get-domain-names", "--output", "json"])
    print(f"Code: {code}")
    print(f"Stdout: {stdout.strip()}")
    print(f"Stderr: {stderr.strip()}")

    print("\n2. Checking API Gateway V1 Domain Names:")
    stdout, stderr, code = run_cmd(["aws", "apigateway", "get-domain-names", "--output", "json"])
    print(f"Code: {code}")
    print(f"Stdout: {stdout.strip()}")
    print(f"Stderr: {stderr.strip()}")

    print("\n3. Checking CloudFront Distributions:")
    stdout, stderr, code = run_cmd(["aws", "cloudfront", "list-distributions", "--output", "json"])
    print(f"Code: {code}")
    if stdout:
        try:
            dists = json.loads(stdout).get("DistributionList", {}).get("Items", [])
            for d in dists:
                aliases = d.get("Aliases", {}).get("Items", [])
                print(f"CF ID: {d.get('Id')}, DomainName: {d.get('DomainName')}, Aliases: {aliases}")
        except Exception as e:
            print(f"Parsing error: {e}")

    print("\n4. Checking ACM Certificates:")
    stdout, stderr, code = run_cmd(["aws", "acm", "list-certificates", "--output", "json"])
    print(f"Code: {code}")
    print(f"Stdout: {stdout.strip()}")

if __name__ == "__main__":
    main()
