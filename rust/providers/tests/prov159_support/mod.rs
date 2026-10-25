use codelet_providers::codex::codex_auth::{CodexAuthJson, CodexTokens};
use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi};
use keyring::mock::MockCredential;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Default)]
pub struct Keyring(Arc<Mutex<HashMap<String, Arc<MockCredential>>>>);

struct Entry(Arc<MockCredential>);

impl CredentialApi for Entry {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        self.0.set_secret(secret)
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        self.0.get_secret()
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        self.0.delete_credential()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl CredentialBuilderApi for Keyring {
    fn build(
        &self,
        _: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        let mut entries = self.0.lock().map_err(|_| {
            keyring::Error::NoStorageAccess(Box::new(std::io::Error::other(
                "isolated keyring lock poisoned",
            )))
        })?;
        let entry = entries.entry(format!("{service}:{user}")).or_default();
        Ok(Box::new(Entry(entry.clone())))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub keyring: Keyring,
    previous: Option<std::ffi::OsString>,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let previous = std::env::var_os("CODEX_HOME");
        std::env::set_var("CODEX_HOME", directory.path());
        let keyring = Keyring::default();
        keyring::set_default_credential_builder(Box::new(keyring.clone()));
        Ok(Self {
            directory,
            keyring,
            previous,
        })
    }

    pub fn keychain(&self) -> std::io::Result<Arc<MockCredential>> {
        use sha2::{Digest, Sha256};
        let home = self.directory.path().canonicalize()?;
        let hash = format!("{:x}", Sha256::digest(home.to_string_lossy().as_bytes()));
        let key = format!("Codex Auth:cli|{}", &hash[..16]);
        Ok(self
            .keyring
            .0
            .lock()
            .map_err(|_| std::io::Error::other("isolated keyring lock poisoned"))?
            .entry(key)
            .or_default()
            .clone())
    }

    pub fn seed(&self, account: &str) -> TestResult {
        let document = serde_json::json!({
            "tokens": tokens(account), "OPENAI_API_KEY": "sk-independent",
            "last_refresh": chrono::Utc::now().to_rfc3339(), "unrelated": {"keep": true}
        });
        std::fs::write(
            self.directory.path().join("auth.json"),
            document.to_string(),
        )?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("CODEX_HOME", previous),
            None => std::env::remove_var("CODEX_HOME"),
        }
    }
}

pub fn tokens(account: &str) -> CodexTokens {
    CodexTokens {
        id_token: format!("id-{account}"),
        access_token: format!("access-{account}"),
        refresh_token: format!("refresh-{account}"),
        account_id: account.to_owned(),
    }
}

pub fn auth(account: &str) -> CodexAuthJson {
    CodexAuthJson {
        openai_api_key: None,
        tokens: Some(tokens(account)),
        last_refresh: Some(chrono::Utc::now().to_rfc3339()),
    }
}
