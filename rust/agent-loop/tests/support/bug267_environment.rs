use std::{ffi::OsString, path::PathBuf};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Environment {
    pub directory: tempfile::TempDir,
    prior_directory: PathBuf,
    variables: Vec<(&'static str, Option<OsString>)>,
}

impl Environment {
    pub fn new() -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let prior_directory = std::env::current_dir()?;
        let variables = [
            "HOME",
            "FSPEC_HOME",
            "FSPEC_USER_DIR",
            "CODEX_HOME",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "ANTHROPIC_API_KEY",
            "FSPEC_DISABLE_SCRIPT_SHADOWING",
        ]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
        let environment = Self {
            directory,
            prior_directory,
            variables,
        };
        let root = environment.directory.path();
        std::env::set_var("HOME", root);
        std::env::set_var("FSPEC_HOME", root.join(".fspec/credentials"));
        std::env::set_var("FSPEC_USER_DIR", root.join(".fspec"));
        std::env::set_var("CODEX_HOME", root.join("codex"));
        std::env::set_var("OPENAI_API_KEY", "synthetic-loopback-only");
        std::env::set_var("ANTHROPIC_API_KEY", "synthetic-loopback-only");
        std::env::remove_var("FSPEC_DISABLE_SCRIPT_SHADOWING");
        std::env::set_current_dir(root)?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(root.to_owned())?;
        Ok(environment)
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior_directory);
        for (key, prior) in self.variables.iter().rev() {
            match prior {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        codelet_core::persistence::reset_stores_for_tests();
    }
}
