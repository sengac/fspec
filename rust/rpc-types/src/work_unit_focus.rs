use crate::WorkUnitContext;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi_derive::napi(object, object_from_js = false))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnitFocus {
    pub revision: u64,
    pub context: Option<WorkUnitContext>,
    pub source: String,
}

impl Default for WorkUnitFocus {
    fn default() -> Self {
        Self {
            revision: 0,
            context: None,
            source: "initial".into(),
        }
    }
}

impl WorkUnitFocus {
    pub fn apply_if_newer(&mut self, incoming: Self) -> bool {
        if incoming.revision <= self.revision {
            return false;
        }
        *self = incoming;
        true
    }
}
