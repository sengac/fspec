use std::{ffi::OsString, path::Path};

pub struct TestEnvironment(Vec<(&'static str, Option<OsString>)>);

impl TestEnvironment {
    pub fn install(directory: &Path) -> Self {
        let environment = Self(
            ["FSPEC_USER_DIR", "OPENAI_API_KEY"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect(),
        );
        std::env::set_var("FSPEC_USER_DIR", directory);
        std::env::set_var("OPENAI_API_KEY", "offline-test-key");
        environment
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        codelet_core::persistence::reset_stores_for_tests();
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
