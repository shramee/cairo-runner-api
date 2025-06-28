use axum::{extract::Json as ExtractJson, http::StatusCode, response::Json};
use cairo_runners::test_runner::run_cairo_tests;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TestRequest {
    cairo_code: String,
}

#[derive(Serialize)]
pub struct TestResponse {
    message: String,
    success: bool,
}

pub async fn test_handler(
    ExtractJson(request): ExtractJson<TestRequest>,
) -> Result<Json<TestResponse>, StatusCode> {
    let response = match run_cairo_tests(request.cairo_code.to_string()) {
        Ok(message) => TestResponse {
            message: format!("{}", message.notes()),
            success: true,
        },
        Err(message) => TestResponse {
            message: format!("{}", message),
            success: true,
        },
    };

    Ok(Json(response))
}
