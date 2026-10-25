#[path = "bug267_environment.rs"]
mod shared;

pub use shared::TestResult;

pub struct Environment {
    inner: shared::Environment,
    variables: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Environment {
    pub fn new() -> TestResult<Self> {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let variables = [
            "CLAUDE_CODE_OAUTH_TOKEN",
            "GOOGLE_GENERATIVE_AI_API_KEY",
            "ZAI_API_KEY",
            "ZAI_PLAN_API_KEY",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "NO_PROXY",
            "no_proxy",
        ]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect::<Vec<_>>();
        let inner = shared::Environment::new()?;
        for (key, _) in &variables {
            std::env::remove_var(key);
        }
        std::env::set_var("OPENAI_BASE_URL", "http://127.0.0.1:9");
        std::env::set_var("NO_PROXY", "*");
        std::env::set_var("no_proxy", "*");
        Ok(Self { inner, variables })
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (key, prior) in self.variables.iter().rev() {
            match prior {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

impl std::ops::Deref for Environment {
    type Target = shared::Environment;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
