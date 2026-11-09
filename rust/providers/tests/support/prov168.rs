use codelet_providers::claude::{AuthMode, ClaudeProvider};
use codelet_test_helpers::setup_temp_directory;
use std::ffi::OsString;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Environment {
    saved: Vec<(&'static str, Option<OsString>)>,
    pub directory: tempfile::TempDir,
}

impl Environment {
    pub fn new() -> TestResult<Self> {
        let saved = ["FSPEC_CLAUDE_CODE_VERSION", "PATH"]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        let directory = setup_temp_directory()?;
        std::env::remove_var("FSPEC_CLAUDE_CODE_VERSION");
        std::env::set_var("PATH", directory.path());
        Ok(Self { saved, directory })
    }

    pub fn install(&self, version: &str) -> TestResult {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let executable = self.directory.path().join("claude");
            std::fs::write(
                &executable,
                format!("#!/bin/sh\nprintf '%s\\n' '{version} (Claude Code)'\n"),
            )?;
            std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(windows)]
        std::fs::write(
            self.directory.path().join("claude.cmd"),
            format!("@echo {version} (Claude Code)\r\n"),
        )?;
        Ok(())
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

pub fn oauth() -> Result<ClaudeProvider, codelet_providers::ProviderError> {
    ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-oat01-prov168",
        AuthMode::OAuth,
        "claude-opus-5-5",
    )
}

pub fn refreshing() -> Result<ClaudeProvider, codelet_providers::ProviderError> {
    ClaudeProvider::from_oauth_tokens(
        "sk-ant-oat01-prov168",
        "refresh-prov168",
        Some(3600),
        "http://127.0.0.1:9",
        "claude-opus-5-5",
    )
}
