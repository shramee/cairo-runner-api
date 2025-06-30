# cairo-runner-api

A server runtime for executing [Cairo](https://www.cairo-lang.org/) code, with REST API implementations for AWS Lambda (Rust) and Axum (Rust web framework).

## Overview

**cairo-runner-api** provides a server-based solution for running Cairo programs,

- **Runners:** Call Cairo compiler, runner and tester functions.
- **AWS Lambda Integration:** Deploy Cairo execution API as a serverless Lambda function.
- **Axum Web Server:** Expose Cairo execution as a web API.

## Project Structure

- `/axum` (if exists) - Axum API server for Cairo execution.
- `/cairo` - Cairo code or related assets.
- `/corelib` - Cairo core lib code for db.
- `/lambda` - AWS Lambda Rust implementation and deployment configs.
- `/types` - Shared response/request types

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html) (for AWS Lambda deployment)

### Building

To build the AWS Lambda project:

```bash
cargo lambda build --release
```

Omit `--release` for development builds.

For the Axum server, use standard Rust cargo commands:

```bash
cargo build --bin cairo-runner-lambda --release
```

### Testing

Unit tests:  
  ```bash
  cargo test
  ```

### Deploying (AWS Lambda)

Deploy AWS lambda function:
:warning: This requires aws cli setup in addition to cargo lambda.
The authenticated IAM role need user needs permissions beyond PowerUser, tested with admin permissions, but could use finegrained perms.

```bash
cargo lambda deploy --bin cairo-runner-lambda
```

## Cairo runner functions usage

#### Available functions and types
```rust
pub struct TestsSummary {
    passed: Vec<String>,
    failed: Vec<String>,
    failed_run_results: Vec<RunResultValue>,
    notes: String,
}

pub fn run_cairo_tests(code: String) -> anyhow::Result<TestsSummary>;
pub fn run_cairo_code(code: String) -> anyhow::Result<String>;
```

#### Usage

```rust
use cairo_runners::{main_runner::run_cairo_code, test_runner::run_cairo_tests};

// Main runner

let code = r#"
fn main() -> u8 {
  4
}"#;

let output = run_cairo_code_string_output(code.to_string());


// Tests runner

let code = r#"
#[test]
fn test_pass() {
  asserts(true, 'should pass');
}
#[test]
fn test_fail() {
  assert(false, 'should fail');
}"#;

let output = run_cairo_code_string_output(code.to_string());

```

## API Usage

#### Lambda URL Example

```bash
curl --location 'https://<lambda-url>' \
--header 'Content-Type: application/json' \
--data '{
    "code": "fn main() -> u128 {1}",
    "test": false
}'
```

```bash
curl --location 'https://<lambda-url>' \
--header 'Content-Type: application/json' \
--data '{
    "code": "#[test]fn test_pass() {asserts(true, \'should pass\');}#[test]fn test_fail() {assert(false, \'should fail\');}",
    "test": true
}'
```

#### HTTP (Axum) Example

```bash
curl --location 'https://<api-url>/test' \
--header 'Content-Type: application/json' \
--data '{
    "code": "#[test]fn test_pass() {asserts(true, \'should pass\');}#[test]fn test_fail() {assert(false, \'should fail\');}"
}'
```

```bash
curl --location 'https://<api-url>/run' \
--header 'Content-Type: application/json' \
--data '{
    "code": "fn main() -> u128 {1}"
}'
```
