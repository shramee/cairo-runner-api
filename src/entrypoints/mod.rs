use axum::{extract::Json as ExtractJson, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Deserialize, Debug)]
pub enum CompilationMode {
    Compile,
    Test,
}

#[derive(Deserialize)]
pub struct CompileRequest {
    cairo_code: String,
    mode: Option<CompilationMode>,
    starknet: Option<bool>,
}

#[derive(Serialize)]
pub struct CompileResponse {
    message: String,
    success: bool,
}

pub async fn compile_handler(
    ExtractJson(request): ExtractJson<CompileRequest>,
) -> Result<Json<CompileResponse>, StatusCode> {
    info!("Compile endpoint called");
    info!("Cairo code: {}", request.cairo_code);

    info!(
        "Compilation op, mode={:?}, starknet={:?}",
        request.mode, request.starknet
    );

    let response = CompileResponse {
        message: "Cairo code received for compilation".to_string(),
        success: true,
    };

    Ok(Json(response))
}

pub async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}
