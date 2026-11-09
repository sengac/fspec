// MODEL-016: the selected local profile model's declared reasoning, kept on
// the ProviderManager next to the profile name. Included by manager.rs.

/// MODEL-016: what the selected local profile model declares. Reset by every
/// profile selection; set by the caller that read the profile.
#[derive(Debug, Clone)]
struct ProfileReasoning {
    /// `Ok(None)`: the model declares nothing, so it has no reasoning control.
    declared: Result<Option<crate::local_reasoning::LocalReasoning>, String>,
    live_check: Option<crate::local_reasoning::LiveCheck>,
}

impl Default for ProfileReasoning {
    fn default() -> Self {
        Self {
            declared: Ok(None),
            live_check: None,
        }
    }
}

impl ProviderManager {
    /// MODEL-016: record what the selected profile model declares, as
    /// resolved by the caller that read its profile, and the live check to
    /// run against its engine. Every profile selection resets both.
    pub fn set_profile_reasoning(
        &mut self,
        declared: Result<Option<crate::local_reasoning::LocalReasoning>, String>,
        live_check: Option<crate::local_reasoning::LiveCheck>,
    ) {
        self.profile_reasoning = ProfileReasoning {
            declared,
            live_check,
        };
    }

    /// MODEL-016: the selected profile model's declared reasoning;
    /// `Ok(None)` when it declares none (no reasoning control), an error
    /// when its declaration cannot be used.
    pub fn local_reasoning(&self) -> &Result<Option<crate::local_reasoning::LocalReasoning>, String> {
        &self.profile_reasoning.declared
    }

    /// MODEL-016: the selected profile model's live check, once: the first
    /// turn after the selection runs it.
    pub fn take_live_check(&mut self) -> Option<crate::local_reasoning::LiveCheck> {
        self.profile_reasoning.live_check.take()
    }
}
