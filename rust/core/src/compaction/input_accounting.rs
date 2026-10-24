use super::TokenTracker;
use crate::token_usage::ApiTokenUsage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputAccounting {
    #[default]
    Uninitialized,
    Unknown,
    Reported,
    Unavailable,
    Restored,
    Reset,
    Estimated,
}

#[derive(Default)]
pub struct InputUsageObservation {
    latest: Option<ApiTokenUsage>,
}

impl InputUsageObservation {
    pub fn observe(&mut self, tracker: &mut TokenTracker, usage: &rig::completion::Usage) {
        let mut observed = ApiTokenUsage::default();
        observed.update_from_usage(usage);
        if observed.total_input() > 0 || self.latest.is_none() {
            tracker.observe_input_usage(&observed);
            self.latest = Some(observed);
        }
    }

    pub fn finish(&mut self, tracker: &mut TokenTracker, usage: &rig::completion::Usage) {
        let mut observed = ApiTokenUsage::default();
        observed.update_from_usage(usage);
        let streamed = self.latest.take();
        if observed.total_input() == 0 {
            observed = streamed.unwrap_or(observed);
        }
        tracker.observe_input_usage(&observed);
    }
}

impl InputAccounting {
    pub fn unknown() -> Self {
        Self::Unknown
    }
}

impl TokenTracker {
    pub fn update_display_only(&mut self, usage: &ApiTokenUsage, cumulative_output: u64) {
        if matches!(
            self.input_accounting,
            InputAccounting::Unknown | InputAccounting::Uninitialized
        ) {
            self.input_tokens = usage.total_input();
            self.cache_read_input_tokens = Some(usage.cache_read_input_tokens);
            self.cache_creation_input_tokens = Some(usage.cache_creation_input_tokens);
        }
        self.output_tokens = cumulative_output;
        self.reasoning_tokens = usage.reasoning_tokens;
    }

    pub fn observe_input_usage(&mut self, usage: &ApiTokenUsage) {
        self.input_tokens = usage.total_input();
        self.cache_read_input_tokens = Some(usage.cache_read_input_tokens);
        self.cache_creation_input_tokens = Some(usage.cache_creation_input_tokens);
        self.input_accounting = if self.input_tokens > 0 {
            InputAccounting::Reported
        } else {
            InputAccounting::Unavailable
        };
    }

    pub fn update_from_display(&mut self, usage: &ApiTokenUsage, cumulative_output: u64) {
        self.update_display_only(usage, cumulative_output);
        self.cumulative_billed_input += usage.input_tokens;
        self.cumulative_billed_output += usage.output_tokens;
    }

    pub fn set_estimated_input(&mut self, tokens: u64) {
        self.input_tokens = tokens;
        self.output_tokens = 0;
        self.cache_read_input_tokens = None;
        self.cache_creation_input_tokens = None;
        self.input_accounting = InputAccounting::Estimated;
    }

    pub fn invalidate_input(&mut self) {
        self.input_tokens = 0;
        self.cache_read_input_tokens = None;
        self.cache_creation_input_tokens = None;
        self.input_accounting = InputAccounting::Reset;
    }
}
