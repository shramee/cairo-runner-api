# cairo-runner-api

A server runtime for executing [Cairo](https://www.cairo-lang.org/) code, with REST API implementations for AWS Lambda (Rust) and Axum (Rust web framework).

---

## Overview

**cairo-runner-api** provides a server-based solution for running Cairo programs,

- **Runners:** Call Cairo compiler, runner and tester functions.
- **AWS Lambda Integration:** Deploy Cairo execution API as a serverless Lambda function.
- **Axum Web Server:** Expose Cairo execution as a web API.
---

## Project Structure

- `/axum` (if exists) - Axum API server for Cairo execution.
- `/cairo` - Cairo code or related assets.
- `/corelib` - Cairo core lib code for db.
- `/lambda` - AWS Lambda Rust implementation and deployment configs.
- `/types` - Shared response/request types
