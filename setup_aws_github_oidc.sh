#!/usr/bin/env bash
set -e

ROLE_NAME="GitHubActionsEduxalSamDeployRole"
GITHUB_REPO="eduxal-labs/microservices"

echo "=== 1. Checking AWS CLI Identity ==="
ACCOUNT_ID=$(aws sts get-caller-identity --query "Account" --output text)
echo "AWS Account ID: ${ACCOUNT_ID}"

echo "=== 2. Creating / Ensuring GitHub OIDC Provider ==="
OIDC_ARN="arn:aws:iam::${ACCOUNT_ID}:oidc-provider/token.actions.githubusercontent.com"

if ! aws iam get-open-id-connect-provider --open-id-connect-provider-arn "${OIDC_ARN}" >/dev/null 2>&1; then
    echo "Creating OIDC Provider..."
    aws iam create-open-id-connect-provider \
      --url "https://token.actions.githubusercontent.com" \
      --client-id-list "sts.amazonaws.com" \
      --thumbprint-list "6938fd4d98bab03faadb97b34396831e3780aea1"
else
    echo "OIDC Provider already exists."
fi

echo "=== 3. Creating IAM Trust Policy for Repository: ${GITHUB_REPO} ==="
cat <<EOF > /tmp/github-trust-policy.json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "${OIDC_ARN}"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:${GITHUB_REPO}:*"
        }
      }
    }
  ]
}
EOF

echo "=== 4. Creating IAM Role: ${ROLE_NAME} ==="
if ! aws iam get-role --role-name "${ROLE_NAME}" >/dev/null 2>&1; then
    aws iam create-role \
      --role-name "${ROLE_NAME}" \
      --assume-role-policy-document file:///tmp/github-trust-policy.json
else
    echo "Role ${ROLE_NAME} exists. Updating trust policy..."
    aws iam update-assume-role-policy \
      --role-name "${ROLE_NAME}" \
      --policy-document file:///tmp/github-trust-policy.json
fi

echo "=== 5. Attaching Deployment Policy ==="
aws iam attach-role-policy \
  --role-name "${ROLE_NAME}" \
  --policy-arn "arn:aws:iam::aws:policy/AdministratorAccess"

rm -f /tmp/github-trust-policy.json

ROLE_ARN="arn:aws:iam::${ACCOUNT_ID}:role/${ROLE_NAME}"

echo ""
echo "=================================================================="
echo " SUCCESS! AWS OIDC & IAM Role Setup Complete."
echo "=================================================================="
echo "Add the following Repository Variables in GitHub:"
echo "Repo URL: https://github.com/${GITHUB_REPO}/settings/variables/actions"
echo ""
echo "  AWS_ROLE_TO_ASSUME = ${ROLE_ARN}"
echo "  AWS_REGION         = us-east-1"
echo "=================================================================="
