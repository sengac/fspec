use crate::ContextFillInfo;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextProvenance {
    Measured,
    Estimated,
    Unknown,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object, object_from_js = false))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContextSnapshot {
    pub model: String,
    pub generation: i64,
    pub request: u64,
    #[serde(default)]
    pub physical_request: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub capacity: Option<u64>,
    pub threshold: Option<u64>,
    pub provenance: ContextProvenance,
}

impl RequestContextSnapshot {
    pub fn context_fill(&self) -> Option<ContextFillInfo> {
        if self.provenance == ContextProvenance::Unknown || self.model.is_empty() {
            return None;
        }
        let capacity = self.capacity.filter(|capacity| *capacity > 0)?;
        let threshold = self
            .threshold
            .filter(|threshold| *threshold > 0 && *threshold <= capacity)?;
        let occupancy = self.input_tokens?.checked_add(self.output_tokens?)?;
        let percentage = u128::from(occupancy) * 100 / u128::from(threshold);
        Some(ContextFillInfo {
            fill_percentage: u32::try_from(percentage).unwrap_or(u32::MAX),
            effective_tokens: occupancy as f64,
            threshold: threshold as f64,
            context_window: capacity as f64,
        })
    }
}
