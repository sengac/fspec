//! Errors produced by the custom provider loader and Rhai script compiler
//! (PROV-062).

use std::path::PathBuf;

use thiserror::Error;

/// All failure modes for the custom provider loader and script compiler.
///
/// Display messages are intentionally descriptive because integration tests
/// assert on substrings such as the allowed name pattern, the resolved
/// script path, and Rhai line/column numbers.
#[derive(Debug, Error)]
pub enum CustomProviderError {
    /// Filesystem I/O failure when reading a config or script file.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// Path being read when the error occurred.
        path: PathBuf,
        /// Underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// JSON could not be deserialized into a [`crate::custom::ProviderConfig`].
    ///
    /// This is also emitted when a required field is missing — serde
    /// surfaces the missing field name in its message, so callers can
    /// assert on the field name substring.
    #[error("failed to parse provider config at {path}: {message}")]
    Parse {
        /// Path of the JSON file being parsed.
        path: PathBuf,
        /// Serde message — contains the offending field name when a
        /// required field is missing.
        message: String,
    },

    /// A required configuration field was missing.
    ///
    /// Kept distinct from [`CustomProviderError::Parse`] so downstream
    /// tooling can render it differently if desired.
    #[error("missing required field '{field}' in provider config at {path}")]
    MissingField {
        /// Path of the offending config file.
        path: PathBuf,
        /// Name of the missing field.
        field: String,
    },

    /// Provider name collides with a built-in provider name.
    #[error(
        "provider name '{name}' in {path} conflicts with built-in provider \
         (built-ins: claude, openai, codex, gemini, zai, github-copilot, copilot)"
    )]
    NameConflict {
        /// Offending provider name.
        name: String,
        /// Path of the config file that declared the name.
        path: PathBuf,
    },

    /// Provider name did not match the allowed identifier pattern.
    #[error("provider name '{name}' in {path} is invalid; allowed pattern ^[a-z][a-z0-9-]*$")]
    InvalidName {
        /// Offending provider name.
        name: String,
        /// Path of the config file that declared the name.
        path: PathBuf,
    },

    /// The `script` field referenced a file that does not exist on disk.
    #[error("script file not found: {resolved_path} (referenced from {config_path})")]
    ScriptNotFound {
        /// Resolved absolute (or best-effort relative) path to the
        /// missing `.rhai` file.
        resolved_path: PathBuf,
        /// Path of the config file that referenced the missing script.
        config_path: PathBuf,
    },

    /// `defaults.model` referenced a model alias that is not present in the
    /// `models` map.
    #[error("default model '{model}' not found in models map for provider '{provider}' at {path}")]
    MissingDefaultModel {
        /// Name of the provider.
        provider: String,
        /// Offending default model alias.
        model: String,
        /// Path of the config file.
        path: PathBuf,
    },

    /// The Rhai engine failed to parse a `.rhai` script. Carries file
    /// path plus line/column info extracted from the underlying
    /// `ParseError` so error messages are useful to plugin authors.
    #[error("failed to compile script {path} at line {line}, column {column}: {message}")]
    RhaiParseError {
        /// Path of the `.rhai` file that failed to parse.
        path: PathBuf,
        /// 1-based line number of the parse error.
        line: usize,
        /// 1-based column number of the parse error (0 when unknown).
        column: usize,
        /// Human-readable parse error message from Rhai.
        message: String,
    },

    /// A compiled script did not define a required provider-lifecycle
    /// function. The message names the missing function so tests (and
    /// humans) can pinpoint the problem.
    #[error("script {path} is missing required function '{function}'")]
    MissingFunction {
        /// Path of the offending script.
        path: PathBuf,
        /// Name of the missing function.
        function: String,
    },

    /// A Rhai script raised a runtime error or produced a value of the
    /// wrong shape. The embedded message preserves the original Rhai
    /// error text so callers can surface it to plugin authors.
    #[error("rhai runtime error: {0}")]
    RhaiRuntimeError(String),
}

impl From<CustomProviderError> for crate::error::ProviderError {
    /// Map the loader-layer errors onto the provider-layer error type.
    ///
    /// - [`CustomProviderError::MissingFunction`] and the
    ///   parse/validation variants become `Configuration` errors so
    ///   the consumer sees them as "your script is wrong".
    /// - [`CustomProviderError::RhaiRuntimeError`] becomes an `Api`
    ///   error, matching the behaviour described by the PROV-063
    ///   research doc.
    fn from(err: CustomProviderError) -> Self {
        use crate::error::ProviderError;
        match err {
            CustomProviderError::RhaiRuntimeError(message) => ProviderError::Api {
                provider: "custom".to_string(),
                message,
            },
            CustomProviderError::MissingFunction { function, .. } => ProviderError::Configuration {
                provider: "custom".to_string(),
                message: format!("script missing required function '{function}'"),
            },
            other => ProviderError::Configuration {
                provider: "custom".to_string(),
                message: other.to_string(),
            },
        }
    }
}
