# Eduxal Microservices Workspace

Multi-service serverless architecture for **eduxal**, managed under a single AWS SAM application and configured as a Rust Cargo Workspace.

## Directory Structure

```text
.
├── template.yaml            # AWS SAM Template defining application resources
├── samconfig.toml           # SAM CLI configuration
├── Cargo.toml               # Root Cargo workspace manifest
├── services/                # Lambda Microservices (Binary Crates)
└── shared/                  # Shared libraries across microservices
```

## Adding Your First Microservice

1. **Create crate under `services/`**:
   ```bash
   cargo new services/<service-name> --bin
   ```

2. **Add crate to `Cargo.toml` `members`**:
   ```toml
   [workspace]
   resolver = "2"
   members = [
       "services/<service-name>",
   ]
   ```

3. **Configure `services/<service-name>/Cargo.toml`**:
   ```toml
   [package]
   name = "<service-name>"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   lambda_runtime = { workspace = true }
   aws_lambda_events = { workspace = true }
   tokio = { workspace = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   tracing = { workspace = true }
   tracing-subscriber = { workspace = true }
   ```

4. **Register in `template.yaml`**:
   ```yaml
   Resources:
     <ServiceName>Function:
       Type: AWS::Serverless::Function
       Properties:
         CodeUri: ./services/<service-name>
         Handler: bootstrap
         Events:
           ApiEvent:
             Type: Api
             Properties:
               Path: /<endpoint>
               Method: get
       Metadata:
         BuildMethod: cargo-lambda
   ```

## Workflow Commands

- **Build / Test Cargo Workspace**:
  ```bash
  cargo check
  ```

- **Validate SAM Template**:
  ```bash
  sam validate
  ```

- **Build & Deploy SAM Application**:
  ```bash
  sam build
  sam deploy --guided
  ```
