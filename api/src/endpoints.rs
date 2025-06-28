use axum::{extract::Json as ExtractJson, http::StatusCode, response::Json};
use cairo_runners::{main_runner::run_cairo_code, test_runner::run_cairo_tests};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CodeRunRequest {
    cairo_code: String,
}

#[derive(Serialize)]
pub struct RunResponse {
    message: String,
    success: bool,
}

pub async fn run_handler(
    ExtractJson(request): ExtractJson<CodeRunRequest>,
) -> Result<Json<RunResponse>, StatusCode> {
    let response = match run_cairo_code(request.cairo_code) {
        Ok(message) => RunResponse {
            message,
            success: true,
        },
        Err(message) => RunResponse {
            message: format!("{}", message),
            success: false,
        },
    };

    Ok(Json(response))
}

pub async fn test_handler(
    ExtractJson(request): ExtractJson<CodeRunRequest>,
) -> Result<Json<RunResponse>, StatusCode> {
    let response = match run_cairo_tests(request.cairo_code.to_string()) {
        Ok(message) => RunResponse {
            message: format!("{}", message.notes()),
            success: true,
        },
        Err(message) => RunResponse {
            message: format!("{}", message),
            success: false,
        },
    };

    Ok(Json(response))
}
