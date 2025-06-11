use axum::{
    extract::Json as ExtractJson,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, Level};

#[derive(Deserialize, Debug)]
enum CompilationMode {
    Compile,
    Test,
}

#[derive(Deserialize)]
struct CompileRequest {
    cairo_code: String,
    mode: Option<CompilationMode>,
    starknet: Option<bool>,
}

#[derive(Serialize)]
struct CompileResponse {
    message: String,
    success: bool,
}

async fn compile_handler(
    ExtractJson(request): ExtractJson<CompileRequest>,
) -> Result<Json<CompileResponse>, StatusCode> {
    let code_length = request.cairo_code.len();

    info!("Compile endpoint called");
    info!("Cairo code length: {} characters", code_length);

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

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!(
        "Starting Cairo Compilation API v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Build the application with middleware
    let app = Router::new()
        .route("/compile", post(compile_handler))
        .route("/health", get(health_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    // Start the server
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;

    let addr = format!("0.0.0.0:{}", port);
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
