# Agent Workflow & Coding Guidelines

This file serves as the single source of truth for AI agents working in this repository. It defines roles, workflows, testing requirements, and learned conventions.

---

## Roles & Responsibilities

### 1. Implementor Agent
- **Task**: Takes feature requests from the user, implements functionality in Rust / AWS SAM IaC, and writes corresponding unit and integration tests.
- **Rules & Requirements**:
  1. Always consult the **Learned Conventions & Preferences** section in this file before writing code.
  2. Implement features cleanly, ensuring code compiles (`cargo check`) and unit tests pass (`cargo test`).
  3. Validate AWS SAM templates (`sam validate`) if infrastructure changes are made.
  4. Create a clean git commit once feature implementation and tests pass.

### 2. User Refinement (Human Step)
- **Task**: The user reviews and modifies the code produced by the Implementor to match their personal architecture, coding style, and design principles.

### 3. Analyzer Agent
- **Task**: Triggered when requested by the user after manual edits.
- **Rules & Requirements**:
  1. Analyze the git diff between the Implementor's commit and the user's refined version.
  2. Identify patterns, stylistic preferences, architectural decisions, and error-handling idioms introduced by the user.
  3. Append these newly extracted principles as concrete, actionable rules in the **Learned Conventions & Preferences** section below so the Implementor adheres to them in future iterations.

---

## Standard Project Conventions

- **Language & Runtime**: Rust (Edition 2021) targeting AWS Lambda on ARM64 (`provided.al2023`).
- **Workspace Structure**:
  - `services/`: Binary crates representing microservices / Lambda functions.
  - `shared/`: Library crates shared across microservices.
- **Testing**:
  - Unit tests placed in `src/` or dedicated `tests/` folders for each crate.
  - Run `cargo test` to verify logic.
- **Git Commits**:
  - Use conventional commits format (e.g. `feat: ...`, `fix: ...`, `refactor: ...`).

---

## Learned Conventions & Preferences

> *This section is dynamically updated by the **Analyzer Agent** after user code refinements.*

### Architecture & Patterns
- **Concrete Database Backends**: Target specific database backends (e.g., `diesel::sqlite::Sqlite` for Diesel) explicitly rather than using generic `DB: Backend` bounds in `ToSql`/`FromSql` trait implementations.
- **Zero-Copy Byte References**: Provide helper methods like `as_bytes_ref(&self) -> &[u8; 12]` on newtype wrappers (e.g., `Id`) to return zero-copy byte slice references with correct lifetime bounds (`'b`).
- **Explicit Feature Dependencies**: When utilizing database-specific capabilities, enable the required crate features explicitly in `Cargo.toml` (e.g., `diesel = { version = "...", optional = true, features = ["sqlite"] }`).
- **Test Feature Aggregator**: Define a `test` feature flag in `Cargo.toml` (e.g., `test = ["diesel", "dynamodb"]`) that enables all optional crate features together, allowing validation via `cargo check -p <crate> -F test`.

### Error Handling & Types
- **Unified Error Enum**: Prefer a centralized `Error` enum in `types::error` (`shared/common/src/types/error.rs`) over micro-enums per struct.
- **Feature-Gated Error Variants**: Use `#[cfg(feature = "...")]` on specific variants inside the central `Error` enum when they depend on optional features.
- **Error Conversions**: Implement `From` traits on `Error` (e.g., `From<bson::oid::Error> for Error`) so `FromStr` and `TryFrom` can return `Result<T, Error>` directly without ad-hoc closures.

### Code Style & Formatting
- **Trait Errors**: `FromStr` and `TryFrom` trait implementations should set `type Err = Error;` for consistent error handling across domain types.

### AWS SAM & Infrastructure
- *(No custom rules recorded yet)*
