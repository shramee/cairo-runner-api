use axum::{extract::Json as ExtractJson, http::StatusCode, response::Json};
use cairo_runner_types::{CairoRunRequest, CairoRunResponse};
use cairo_runners::{main_runner::run_cairo_code, test_runner::run_cairo_tests};

pub async fn run_handler(
    ExtractJson(request): ExtractJson<CairoRunRequest>,
) -> Result<Json<CairoRunResponse>, StatusCode> {
    let response = match run_cairo_code(request.cairo_code) {
        Ok(message) => CairoRunResponse {
            message,
            success: true,
        },
        Err(message) => CairoRunResponse {
            message: format!("{}", message),
            success: false,
        },
    };

    Ok(Json(response))
}

pub async fn test_handler(
    ExtractJson(request): ExtractJson<CairoRunRequest>,
) -> Result<Json<CairoRunResponse>, StatusCode> {
    let response = match run_cairo_tests(request.cairo_code.to_string()) {
        Ok(message) => CairoRunResponse {
            message: format!("{}", message.notes()),
            success: true,
        },
        Err(message) => CairoRunResponse {
            message: format!("{}", message),
            success: false,
        },
    };

    Ok(Json(response))
}
