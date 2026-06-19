//! RPC-054 — DetailStatus enum + Span mapper.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! Extracted from `mod.rs` to keep that file under the 300-LoC
//! source-shape ceiling. The Spans are used by `detail::render_detail`
//! to paint the per-status line inside Detail::Summary.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Per-provider status displayed inside Detail::Summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailStatus {
    Testing,
    TestOk { latency_ms: u32 },
    TestErr { error: String },
    RefreshingModels,
    ModelsRefreshed { count: u32 },
    SavingCredentials,
    CredentialsSaved,
    Error { message: String },
}

impl DetailStatus {
    pub fn to_span(&self) -> Span<'static> {
        match self {
            DetailStatus::Testing => Span::styled("Testing…", Style::default().fg(Color::Cyan)),
            DetailStatus::TestOk { latency_ms } => Span::styled(
                format!("✓ ok ({latency_ms}ms)"),
                Style::default().fg(Color::Green),
            ),
            DetailStatus::TestErr { error } => {
                Span::styled(format!("✗ {error}"), Style::default().fg(Color::Red))
            }
            DetailStatus::RefreshingModels => {
                Span::styled("Refreshing models…", Style::default().fg(Color::Cyan))
            }
            DetailStatus::ModelsRefreshed { count } => Span::styled(
                format!("✓ models refreshed ({count})"),
                Style::default().fg(Color::Green),
            ),
            DetailStatus::SavingCredentials => {
                Span::styled("Saving credentials…", Style::default().fg(Color::Cyan))
            }
            DetailStatus::CredentialsSaved => {
                Span::styled("✓ credentials saved", Style::default().fg(Color::Green))
            }
            DetailStatus::Error { message } => {
                Span::styled(format!("✗ {message}"), Style::default().fg(Color::Red))
            }
        }
    }
}
