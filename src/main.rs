use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, Level};

#[derive(Serialize)]
struct HelloResponse {
    message: String,
    version: String,
    server_info: ServerInfo,
    timestamp: String,
}

#[derive(Serialize)]
struct ServerInfo {
    name: String,
    description: String,
    capabilities: Vec<String>,
}

#[derive(Deserialize)]
struct HelloQuery {
    name: Option<String>,
}

async fn hello_handler(
    Query(params): Query<HelloQuery>,
) -> Result<Json<HelloResponse>, StatusCode> {
    let name = params.name.unwrap_or_else(|| "World".to_string());

    let response = HelloResponse {
        message: format!("Hello, {}!", name),
        version: env!("CARGO_PKG_VERSION").to_string(),
        server_info: ServerInfo {
            name: "Cairo Compilation API".to_string(),
            description: "High-performance server-side Cairo compilation service".to_string(),
            capabilities: vec![
                "cairo_compilation".to_string(),
                "async_processing".to_string(),
                "result_caching".to_string(),
            ],
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    info!("Hello endpoint called with name: {}", name);
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
        .route("/hello", get(hello_handler))
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
