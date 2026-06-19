//! RPC-158 — Inline test-result types for the provider settings list.
//!
//! Feature: spec/features/rpc158-provider-settings-inline-test-result.feature
//!
//! Mirrors the TS `TestResult` discriminated union in
//! `src/tui/hooks/useProviderSettingsState.ts` (testResult field). The
//! Rust port keeps only the three "test connection" variants — TS's
//! other status variants live on `DetailStatus` (status_text.rs) for
//! the legacy Detail::Summary path that RPC-162 will eventually delete.

use ratatui::style::Color;

/// Three-state lifecycle of a single provider connection test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTestStatus {
    /// Round-trip in flight — renders cyan "Testing…".
    Testing,
    /// Round-trip completed successfully — renders green "✓ ok (Nms)".
    Ok { latency_ms: u32 },
    /// Round-trip failed — renders red "✗ <message>".
    Err { message: String },
}

impl ProviderTestStatus {
    /// Decoration payload for the inline row paint: the visible text
    /// (without any leading space — the renderer adds the separator)
    /// and the foreground colour. Bytes-for-bytes identical to the
    /// `DetailStatus::Testing / TestOk / TestErr` rendering in
    /// `status_text.rs`.
    pub fn decoration(&self) -> (String, Color) {
        match self {
            ProviderTestStatus::Testing => ("Testing…".to_string(), Color::Cyan),
            ProviderTestStatus::Ok { latency_ms } => {
                (format!("✓ ok ({latency_ms}ms)"), Color::Green)
            }
            ProviderTestStatus::Err { message } => (format!("✗ {message}"), Color::Red),
        }
    }
}

/// Tuple of `(provider_id, status)` — identifies which row receives the
/// decoration and which lifecycle state to paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderTestResult {
    pub provider_id: String,
    pub status: ProviderTestStatus,
}

use super::ProviderSettingsView;

impl ProviderSettingsView {
    /// RPC-158: store an inline test-result for a single provider row.
    /// Last-write-wins — a second call with the same or different
    /// `provider_id` overrides the prior status. Pure state mutation:
    /// never touches `selected_index`, `scroll_offset`, `mode`,
    /// `filter`, `filter_mode`, `expanded`, `nav_items`, or `status`.
    pub fn set_test_result(&mut self, provider_id: impl Into<String>, status: ProviderTestStatus) {
        self.test_result = Some(ProviderTestResult {
            provider_id: provider_id.into(),
            status,
        });
    }

    /// RPC-158: clear any in-flight or completed test-result decoration.
    /// Pure state mutation — see `set_test_result` for the invariant
    /// list of fields this method never touches.
    pub fn clear_test_result(&mut self) {
        self.test_result = None;
    }
}

impl Default for ProviderSettingsView {
    fn default() -> Self {
        Self::new()
    }
}
