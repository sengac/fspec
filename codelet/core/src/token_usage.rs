//! Token Usage Types (PROV-001)
//!
//! Single source of truth for Anthropic prompt caching token calculations.
//!
//! Per Anthropic docs and confirmed by Letta implementation:
//! "Total input tokens in a request is the summation of
//! input_tokens, cache_creation_input_tokens, and cache_read_input_tokens."
//!
//! These are three DISJOINT sets:
//! - input_tokens: fresh tokens not from cache and not being cached
//! - cache_read_input_tokens: tokens read from existing cache
//! - cache_creation_input_tokens: tokens being written to new cache

/// API token usage from a single request (PROV-001)
///
/// This struct holds the raw API response values and provides
/// methods to calculate totals. Use this as the single source
/// of truth for token calculations.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiTokenUsage {
    /// Raw input tokens from API (non-cached, non-creating portion)
    pub input_tokens: u64,
    /// Tokens read from cache
    pub cache_read_input_tokens: u64,
    /// Tokens being written to cache
    pub cache_creation_input_tokens: u64,
    /// Output tokens from API
    pub output_tokens: u64,
}

impl ApiTokenUsage {
    /// Create new token usage with all values
    pub fn new(
        input_tokens: u64,
        cache_read: u64,
        cache_creation: u64,
        output_tokens: u64,
    ) -> Self {
        Self {
            input_tokens,
            cache_read_input_tokens: cache_read,
            cache_creation_input_tokens: cache_creation,
            output_tokens,
        }
    }

    /// Total input tokens = input + cache_read + cache_creation (PROV-001)
    ///
    /// This is the total context size for input, as per Anthropic docs:
    /// "Total input tokens in a request is the summation of
    /// input_tokens, cache_creation_input_tokens, and cache_read_input_tokens."
    #[inline]
    pub fn total_input(&self) -> u64 {
        self.input_tokens + self.cache_read_input_tokens + self.cache_creation_input_tokens
    }

    /// Total context = total_input + output (for threshold checks)
    #[inline]
    pub fn total_context(&self) -> u64 {
        self.total_input() + self.output_tokens
    }

    /// Update from rig Usage struct
    pub fn update_from_usage(&mut self, usage: &rig::completion::Usage) {
        self.input_tokens = usage.input_tokens;
        self.output_tokens = usage.output_tokens;
        self.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
        self.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
    }

    /// Update cache tokens, keeping existing if new values are None
    pub fn update_cache(&mut self, cache_read: Option<u64>, cache_creation: Option<u64>) {
        if let Some(cr) = cache_read {
            self.cache_read_input_tokens = cr;
        }
        if let Some(cc) = cache_creation {
            self.cache_creation_input_tokens = cc;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_input_calculation() {
        // PROV-001: Verify Anthropic total input formula
        let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
        // Total input = 100k + 50k + 5k = 155k
        assert_eq!(usage.total_input(), 155_000);
    }

    #[test]
    fn test_total_context_includes_output() {
        let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
        // Total context = 155k input + 10k output = 165k
        assert_eq!(usage.total_context(), 165_000);
    }

    #[test]
    fn test_no_cache_scenario() {
        let usage = ApiTokenUsage::new(100_000, 0, 0, 5_000);
        assert_eq!(usage.total_input(), 100_000);
        assert_eq!(usage.total_context(), 105_000);
    }

    #[test]
    fn test_default_is_zero() {
        let usage = ApiTokenUsage::default();
        assert_eq!(usage.total_input(), 0);
        assert_eq!(usage.total_context(), 0);
    }
}
