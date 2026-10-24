use crate::{ContextFillInfo, TokenTracker};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedContextSnapshot {
    pub generation: i64,
    pub tokens: TokenTracker,
    pub context_fill: ContextFillInfo,
    pub original_tokens: u32,
    pub covered_turns: Option<u32>,
    pub retained_turns: Option<u32>,
    pub estimated: bool,
    pub reconciled: bool,
}

impl CommittedContextSnapshot {
    pub fn reduction_percentage(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        (1.0 - self.context_fill.effective_tokens / f64::from(self.original_tokens)) * 100.0
    }
}
