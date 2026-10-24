use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct RateWindow {
    pub used_percent: f64,
    pub window_minutes: Option<u64>,
    pub reset_at: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct RateLimits {
    pub primary: Option<RateWindow>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct CodexMetadataStatus {
    pub request_id: Option<String>,
    pub server_model: Option<String>,
    pub reasoning_included: bool,
    pub rate_limits: BTreeMap<String, RateLimits>,
    pub buffering: bool,
    pub faster_model: Option<String>,
    pub verification_required: bool,
    pub moderation_received: bool,
}
