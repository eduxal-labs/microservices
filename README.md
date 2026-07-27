# Eduxal Microservices Workspace

Serverless microservices architecture for **eduxal**, built in Rust and managed under an AWS SAM application cargo workspace.

---

<details open>
<summary>📂 <b>1. Overview & Workspace Structure</b></summary>

### Directory Structure

```text
.
├── template.yaml            # AWS SAM Template defining application resources
├── samconfig.toml           # SAM CLI configuration
├── Cargo.toml               # Root Cargo workspace manifest
├── services/                # Lambda Microservices (Binary Crates)
│   ├── authentication/      # Axum HTTP Auth microservice (/login, /verify, /setup, /refresh)
│   └── messenger/           # High-concurrency SQS WhatsApp OTP batch consumer
└── shared/                  # Shared domain libraries
    ├── common/              # Types, PASETO token engine, DynamoDB converters, Errors
    └── macros/              # Compile-time key derivation macros
```

</details>

---

<details>
<summary>🔐 <b>2. Authentication Architecture & Flow</b></summary>

### Authentication Sequence Flow

```text
[ Client App ]
      │
      │  1. POST /login { phone }
      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Authentication Microservice (Lambda Function)                         │
│                                                                        │
│ 1. Validates Phone format (E.164 e.g. +254712345678)                   │
│ 2. Queries AWS SQS GetQueueAttributes for MessengerQueue               │
│    └─► Calculates pending = ApproximateNumberOfMessages +              │
│                              ApproximateNumberOfMessagesNotVisible     │
│    └─► Returns HTTP 429 (Too Many Requests) if pending >= 20,000       │
│ 3. Queries eduxal-verifications DynamoDB table for existing record     │
│    └─► Returns Err(Error::SlowDown) if created < RLTS (60s rate limit) │
│ 4. Generates new Verification object (random 6-digit OTP, ttl)         │
│ 5. Saves Verification to eduxal-verifications DynamoDB table           │
│ 6. Enqueues ONLY { "phone": "+254712345678" } to SQS MessengerQueue    │
│ 7. Returns Verification object (skipping `code` via Serde)             │
└────────────────────────────────────────────────────────────────────────┘
      │
      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ AWS SQS MessengerQueue (BatchSize: 80, BatchingWindow: 1s)             │
└────────────────────────────────────────────────────────────────────────┘
      │
      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Messenger Microservice (Lambda Function)                              │
│                                                                        │
│ 1. Receives SqsEvent containing up to 80 messages (phone numbers)      │
│ 2. Extracts phone numbers from SQS record payloads                      │
│ 3. Executes DynamoDB BatchGetItem to fetch all 80 Verification items   │
│    from eduxal-verifications table in a single bulk operation          │
│ 4. Uses reqwest::Client connection pool with async HTTP pipelining      │
│ 5. Dispatches all 80 WhatsApp OTP messages concurrently via           │
│    futures::future::join_all to WhatsApp gateway                       │
│ 6. Returns SqsBatchResponse with batch_item_failures for retry         │
└────────────────────────────────────────────────────────────────────────┘
      │
      ▼
[ User Handset receives WhatsApp OTP Code ]
      │
      │  2. POST /verify { phone, code }
      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Authentication Microservice (Lambda Function)                         │
│                                                                        │
│ 1. Fetches Verification for `phone` from eduxal-verifications table    │
│ 2. Verifies OTP code match & checks !verification.is_expired()         │
│ 3. Deletes Verification from eduxal-verifications DynamoDB table       │
│ 4. Generates Token<Setup> (containing phone)                           │
│ 5. Generates signed Cloudflare R2 upload_url for profile image        │
│ 6. Returns { "token": Token<Setup>, "upload_url": "https://r2..." }    │
└────────────────────────────────────────────────────────────────────────┘
      │
      │ (Client uploads profile image to upload_url)
      │
      │  3. POST /setup { name, device } (Authorization: Bearer <Token<Setup>>)
      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Authentication Microservice (Lambda Function)                         │
│                                                                        │
│ 1. Decrypts & validates Token<Setup> (extracts phone)                  │
│ 2. Creates User record in eduxal-users DynamoDB table (phone, name)    │
│ 3. Creates Session::new(user.id, device) in eduxal-sessions table      │
│ 4. Generates Token<Access>, Token<Refresh>, and signed profile URL     │
│ 5. Returns Bundle { access, refresh, profile }                         │
└────────────────────────────────────────────────────────────────────────┘
      │
      │  4. GET /refresh  (Authorization: Bearer <Token<Refresh>>)
      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Authentication Microservice (Lambda Function)                         │
│                                                                        │
│ 1. Decrypts & validates Token<Refresh> (extracts session.id, user.id)  │
│ 2. Fetches Session from eduxal-sessions DynamoDB table                 │
│ 3. Validates session.status == Active and !session.is_expired()        │
│ 4. Generates new Token<Access> and refreshed Token<Refresh>            │
│ 5. Returns updated Bundle { access, refresh, profile }                 │
└────────────────────────────────────────────────────────────────────────┘
```

</details>

---

<details>
<summary>🚀 <b>3. Service Endpoints & Data Contracts</b></summary>

### A. Endpoint `/login` (`POST`)
- **Body**: `{ "phone": "+254712345678" }`
- **Queue Backpressure**: Queries `MessengerQueue` backlog via `GetQueueAttributes`. Returns HTTP 429 if `ApproximateNumberOfMessages + ApproximateNumberOfMessagesNotVisible >= 20,000`.
- **Response (200 OK)**:
  ```json
  {
    "phone": "+254712345678",
    "created": "2026-07-27T16:00:00Z",
    "ttl": "2026-07-27T16:05:00Z"
  }
  ```
  *(Returns `Verification` struct with `code` skipped via Serde `#[serde(skip_serializing, default)]`)*

### B. Endpoint `/verify` (`POST`)
- **Body**: `{ "phone": "+254712345678", "code": "849201" }`
- **Response**:
  ```json
  {
    "token": "v4.local.encrypted_setup_token_string...",
    "upload_url": "https://r2.eduxal.com/upload/profile_123.jpg?signature=..."
  }
  ```
  *(Returns `Token<Setup>` and a pre-signed Cloudflare R2 upload URL for profile picture)*

### C. Endpoint `/setup` (`POST`)
- **Headers**: `Authorization: Bearer <Token<Setup>>`
- **Body**: `{ "name": "John Doe", "device": "iPhone 15 Pro / iOS 17" }`
- **Response**:
  ```json
  {
    "access": "v4.local.encrypted_access_token_string...",
    "refresh": "v4.local.encrypted_refresh_token_string...",
    "profile": "https://r2.eduxal.com/profiles/user_123.jpg?signature=..."
  }
  ```
  *(Creates `User` and `Session`, returning `Bundle` containing `access`, `refresh`, and signed R2 `profile` image URL)*

### D. Endpoint `/refresh` (`GET` / `POST`)
- **Headers**: `Authorization: Bearer <Token<Refresh>>`
- **Response**:
  ```json
  {
    "access": "v4.local.new_encrypted_access_token_string...",
    "refresh": "v4.local.new_encrypted_refresh_token_string...",
    "profile": "https://r2.eduxal.com/profiles/user_123.jpg?signature=..."
  }
  ```

</details>

---

<details>
<summary>⚡ <b>4. High-Concurrency Messenger Service</b></summary>

1. **Backpressure Protection**:
   - Drops/defers queue overload when pending messages exceed 20,000.
2. **Decoupled Security Pattern**:
   - SQS queue receives **only phone numbers** (`{ "phone": "+254712345678" }`), keeping secret OTP codes inside DynamoDB.
3. **Bulk DynamoDB `BatchGetItem`**:
   - SQS triggers `MessengerFunction` with up to 80 phone numbers per batch.
   - Fetches all 80 `Verification` objects in a single DynamoDB roundtrip.
4. **Concurrent WhatsApp Pipelining**:
   - Async HTTP connection pooling (`pool_max_idle_per_host(80)`).
   - Dispatches all 80 WhatsApp API dispatches concurrently using `futures::future::join_all`.
5. **Partial Batch Retries**:
   - Returns `SqsBatchResponse` with `batch_item_failures`, retrying only failed messages.

</details>

---

<details>
<summary>🛠 <b>5. Workspace Commands & Deployment</b></summary>

### Local Build & Test Commands

```bash
# Check all crates in cargo workspace
cargo check --workspace

# Run all unit tests
cargo test --workspace
```

### AWS SAM Deployment Commands

```bash
# Validate SAM template
sam validate

# Build & Deploy
sam build
sam deploy --guided
```

</details>
