//! Error mapping helpers for the custom provider (PROV-063).
//!
//! These helpers translate Rhai errors and script-returned error maps
//! into [`ProviderError`] variants.

use rhai::{Dynamic, EvalAltResult, Map};

use crate::error::ProviderError;

/// Convert a Rhai [`EvalAltResult`] (raised by `engine.call_fn`) into a
/// [`ProviderError`].
///
/// Script bugs map to `Configuration`, runtime errors map to `Api`, and
/// resource limits map to `Timeout`.
pub(crate) fn map_rhai_error_to_provider(
    provider: &str,
    fn_name: &str,
    error: &EvalAltResult,
) -> ProviderError {
    match error {
        EvalAltResult::ErrorFunctionNotFound(sig, _) => ProviderError::config(
            provider,
            format!("script missing required function '{sig}' ({fn_name})"),
        ),
        EvalAltResult::ErrorParsing(_, _) => ProviderError::config(
            provider,
            format!("script syntax error in '{fn_name}': {error}"),
        ),
        EvalAltResult::ErrorVariableNotFound(var, _) => ProviderError::config(
            provider,
            format!("script variable not found in '{fn_name}': {var}"),
        ),
        EvalAltResult::ErrorTooManyOperations(_) => ProviderError::Timeout {
            provider: provider.to_string(),
            message: format!("script '{fn_name}' exceeded max operations"),
        },
        EvalAltResult::ErrorStackOverflow(_) => ProviderError::Timeout {
            provider: provider.to_string(),
            message: format!("script '{fn_name}' stack overflow"),
        },
        EvalAltResult::ErrorDataTooLarge(typ, _) => ProviderError::Timeout {
            provider: provider.to_string(),
            message: format!("script '{fn_name}' data too large: {typ}"),
        },
        EvalAltResult::ErrorTerminated(_, _) => ProviderError::Timeout {
            provider: provider.to_string(),
            message: format!("script '{fn_name}' was terminated"),
        },
        EvalAltResult::ErrorRuntime(msg, _) => {
            let message = if let Some(s) = msg.read_lock::<String>() {
                s.clone()
            } else if let Some(s) = msg.read_lock::<rhai::ImmutableString>() {
                s.to_string()
            } else {
                msg.to_string()
            };
            ProviderError::api(
                provider,
                format!("script '{fn_name}' runtime error: {message}"),
            )
        }
        EvalAltResult::ErrorInFunctionCall(called, _, inner, _) => {
            let mapped = map_rhai_error_to_provider(provider, called, inner);
            match mapped {
                ProviderError::Api { message, .. } => ProviderError::api(
                    provider,
                    format!("script '{fn_name}' → {called}: {message}"),
                ),
                other => other,
            }
        }
        EvalAltResult::ErrorMismatchDataType(expected, actual, _) => ProviderError::config(
            provider,
            format!("script '{fn_name}' type error: expected {expected}, got {actual}"),
        ),
        _ => ProviderError::api(
            provider,
            format!("script '{fn_name}' failed: {error}"),
        ),
    }
}

/// Convert the Rhai `map_error(status, body)` return value into a
/// [`ProviderError`]. Expected return shape:
///
/// ```rhai
/// #{ type: "auth" | "rate_limit" | "api" | "network" | "config" |
///           "timeout" | "model" | "content" | "unauthorized",
///    message: "...",
///    retry_after_secs: 30 }  // optional
/// ```
///
/// When the script returns an unexpected shape, fall back to HTTP
/// status-code heuristics (401/403 → Auth, 429 → RateLimit, 5xx → Api).
pub(crate) fn dynamic_to_provider_error(
    provider: &str,
    status: u16,
    body: &str,
    value: Dynamic,
) -> ProviderError {
    let map = match value.try_cast::<Map>() {
        Some(m) => m,
        None => return fallback_from_status(provider, status, body),
    };

    let error_type = map
        .get("type")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default();

    let message = map
        .get("message")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .unwrap_or_else(|| default_message(status, body));

    let retry_after = map
        .get("retry_after_secs")
        .and_then(|v| v.as_int().ok())
        .map(|v| v as u64);

    match error_type.as_str() {
        "auth" | "authentication" | "unauthorized" => ProviderError::auth(provider, message),
        "rate_limit" | "rate-limit" | "rate_limited" => {
            ProviderError::rate_limit(provider, message, retry_after)
        }
        "timeout" => ProviderError::Timeout {
            provider: provider.to_string(),
            message,
        },
        "config" | "configuration" => ProviderError::config(provider, message),
        "model" => ProviderError::Model {
            provider: provider.to_string(),
            message,
        },
        "content" => ProviderError::Content {
            provider: provider.to_string(),
            message,
        },
        // "api", "network", unknown, or empty → Api (with status fallback
        // for the unknown case so we still communicate the status class).
        "api" | "network" => ProviderError::api(provider, message),
        _ => {
            tracing::warn!(
                provider,
                error_type = %error_type,
                "map_error returned unknown type; falling back to status heuristic"
            );
            fallback_from_status(provider, status, body)
        }
    }
}

/// Heuristic fallback when the Rhai script does not supply a usable
/// error type.
fn fallback_from_status(provider: &str, status: u16, body: &str) -> ProviderError {
    let message = default_message(status, body);
    match status {
        401 | 403 => ProviderError::auth(provider, message),
        429 => ProviderError::rate_limit(provider, message, None),
        500..=599 => ProviderError::api(provider, message),
        _ => ProviderError::api(provider, message),
    }
}

fn default_message(status: u16, body: &str) -> String {
    if body.is_empty() {
        format!("HTTP {status}")
    } else {
        // Truncate so we don't spill giant error bodies into logs.
        let snippet: String = body.chars().take(512).collect();
        format!("HTTP {status}: {snippet}")
    }
}
