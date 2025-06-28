use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CairoRunRequest {
    pub code: String,
    pub test: Option<bool>,
}

#[derive(Serialize)]
pub struct CairoRunResponse {
    pub message: String,
    pub success: bool,
}
