use axum::{extract::Json as ExtractJson, http::StatusCode, response::Json};
use cairo_runners::runner::run_cairo_code;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RunRequest {
    cairo_code: String,
}

#[derive(Serialize)]
pub struct RunResponse {
    message: String,
    success: bool,
}

pub async fn run_handler(
    ExtractJson(request): ExtractJson<RunRequest>,
) -> Result<Json<RunResponse>, StatusCode> {
    let response = match run_cairo_code(request.cairo_code) {
        Ok(message) => RunResponse {
            message,
            success: true,
        },
        Err(message) => RunResponse {
            message: format!("{}", message),
            success: true,
        },
    };

    Ok(Json(response))
}
