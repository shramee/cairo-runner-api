use crate::cairo::runner::run_cairo_code;
use axum::{extract::Json as ExtractJson, http::StatusCode, response::Json};
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
    let response = match run_cairo_code(request.cairo_code) {
        Ok(message) => TestResponse {
            message,
            success: true,
        },
        Err(message) => TestResponse {
            message: format!("{}", message),
            success: true,
        },
    };

    Ok(Json(response))
}
