//! PROV-101: provider resolution when NONE was explicitly selected.
//!
//! Extracted from `manager.rs` (PROV-101 FIX 3) so the no-silent-default
//! resolution policy lives in a small, focused, independently-testable module.
//!
//! Feature: spec/features/provider-resolution-no-silent-default.feature
//!
//! The pre-PROV-101 `detect_default_provider` priority chain
//! (Claude > Gemini > ZAI > Codex > GitHubCopilot > OpenAI) was a silent
//! selection fallback: with several credentialed providers it always chose
//! Claude. That is removed. The replacement makes no *choice* — it only
//! resolves an UNAMBIGUOUS single-provider state, and otherwise errors so the
//! caller must select a provider explicitly.

use crate::credentials::ProviderCredentials;
use crate::manager::ProviderType;
use crate::ProviderError;

/// Resolve a provider when none was explicitly selected.
///
/// * zero credentialed providers  -> `Err` (the existing auth error)
/// * exactly one                  -> `Ok(that provider)` (unambiguous)
/// * more than one                -> `Err` (ambiguous; none explicitly
///   selected) — the caller must select a provider explicitly.
pub(crate) fn resolve_unambiguous_provider(
    credentials: &ProviderCredentials,
) -> Result<ProviderType, ProviderError> {
    let mut resolved: Option<ProviderType> = None;
    let mut count = 0usize;
    let mut consider = |present: bool, provider: ProviderType| {
        if present {
            count += 1;
            if resolved.is_none() {
                resolved = Some(provider);
            }
        }
    };
    consider(credentials.has_claude(), ProviderType::Claude);
    consider(credentials.has_gemini(), ProviderType::Gemini);
    consider(credentials.has_zai(), ProviderType::ZAI);
    consider(credentials.has_codex(), ProviderType::Codex);
    consider(
        credentials.has_github_copilot(),
        ProviderType::GitHubCopilot,
    );
    consider(credentials.has_openai(), ProviderType::OpenAI);

    match count {
        0 => Err(ProviderError::auth(
            "manager",
            "No provider credentials available",
        )),
        1 => resolved
            .ok_or_else(|| ProviderError::auth("manager", "No provider credentials available")),
        _ => Err(ProviderError::config(
            "manager",
            "Multiple providers are credentialed but none was explicitly selected. \
             Select a provider explicitly (e.g. --provider or --model provider/model-id).",
        )),
    }
}
