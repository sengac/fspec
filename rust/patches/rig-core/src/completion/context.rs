use super::Usage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub model: String,
    pub generation: u64,
    pub request: String,
    pub usage: Option<Usage>,
}
