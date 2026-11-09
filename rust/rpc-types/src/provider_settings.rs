use serde::{Deserialize, Serialize};

use crate::ProfileDefinition;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSettingsConfig {
    pub provider_id: String,
    pub settings: ProfileDefinition,
    pub profiles: Vec<ProviderProfileConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderProfileConfig {
    pub name: String,
    pub definition: ProfileDefinition,
}
