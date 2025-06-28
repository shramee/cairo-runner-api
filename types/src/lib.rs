use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CairoRunRequest {
    pub cairo_code: String,
}

#[derive(Serialize)]
pub struct CairoRunResponse {
    pub message: String,
    pub success: bool,
}
