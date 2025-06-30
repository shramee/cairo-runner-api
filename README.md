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
