use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionManifestIssue {
    pub path: PathBuf,
    pub session_id: Option<String>,
    pub category: String,
    pub reason: String,
    pub recovery: String,
}
