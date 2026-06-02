//! RPC-104 — Glyph constants for the provider-settings row renderer.
//!
//! Mirrors the icon string literals used by the TS
//! `ProviderSettingsPanel` (src/tui/components/ProviderSettingsPanel.tsx
//! lines 591, 592, 654, 694, 734, 766) so the Rust port stays
//! pixel-identical.
//!
//! Every constant includes the trailing space that follows the glyph
//! in the TS source — callers concatenate them verbatim.

/// Expanded provider row marker glyph (▼ + trailing space).
pub const EXPANDED: &str = "▼ ";

/// Collapsed provider row marker glyph (▶ + trailing space).
pub const COLLAPSED: &str = "▶ ";

/// Profile-row icon (📁 + trailing space). Two display cells wide.
pub const FOLDER: &str = "📁 ";

/// OAuth-login row and API-key row icon (🔑 + trailing space).
pub const KEY: &str = "🔑 ";

/// Add-profile row icon (+ + trailing space).
pub const PLUS: &str = "+ ";

/// Four-space inner indent prepended to every non-provider nav-item
/// row after the selection prefix.
pub const INDENT: &str = "    ";

/// Selection prefix painted on the focused row.
pub const SEL: &str = "> ";

/// Selection prefix placeholder painted on unfocused rows (two
/// spaces — same width as `SEL` to keep columns aligned).
pub const NOSEL: &str = "  ";
