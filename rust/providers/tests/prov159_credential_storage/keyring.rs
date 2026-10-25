use super::prov159_support::{auth, Fixture, Keyring, TestResult};
use codelet_providers::codex::codex_auth::{
    clear_codex_oauth, delete_codex_auth, write_codex_auth,
};
use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi};
use std::any::Any;
use std::path::{Path, PathBuf};

struct DeleteFailure(Keyring, Option<PathBuf>);

struct Entry(Box<Credential>, Option<PathBuf>);

impl CredentialApi for Entry {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        self.0.set_secret(secret)
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        self.0.get_secret()
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        if let Some(home) = &self.1 {
            read_only(home).map_err(|error| keyring::Error::PlatformFailure(Box::new(error)))?;
        }
        Err(keyring::Error::NoStorageAccess(Box::new(
            std::io::Error::other("delete denied"),
        )))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl CredentialBuilderApi for DeleteFailure {
    fn build(
        &self,
        target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        Ok(Box::new(Entry(
            self.0.build(target, service, user)?,
            self.1.clone(),
        )))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn read_only(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o500))
}

struct HomePermissions {
    path: PathBuf,
    permissions: std::fs::Permissions,
}

impl HomePermissions {
    fn capture(path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            path: path.to_owned(),
            permissions: std::fs::metadata(path)?.permissions(),
        })
    }
}

impl Drop for HomePermissions {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, self.permissions.clone());
    }
}

pub fn verify_failure_safety(fixture: &Fixture) -> TestResult {
    let path = fixture.directory.path().join("auth.json");
    let keychain = fixture.keychain()?;
    let mut outcomes = Vec::new();
    for fallback in [true, false] {
        for operation in ["save", "clear", "delete"] {
            fixture.seed("B")?;
            if !fallback {
                std::fs::remove_file(&path)?;
            }
            keychain.set_password(&serde_json::to_string(&auth("A"))?)?;
            let before = std::fs::read(&path).ok();
            keyring::set_default_credential_builder(Box::new(DeleteFailure(
                fixture.keyring.clone(),
                None,
            )));
            let result = match operation {
                "save" => write_codex_auth(&auth("C")),
                "clear" => clear_codex_oauth(),
                _ => delete_codex_auth(),
            };
            outcomes.push((
                operation,
                fallback,
                result.is_err() && std::fs::read(&path).ok() == before,
            ));
            let preferred: serde_json::Value = serde_json::from_str(&keychain.get_password()?)?;
            assert_eq!(preferred["tokens"]["account_id"], "A");
        }
    }

    fixture.seed("B")?;
    keyring::set_default_credential_builder(Box::new(fixture.keyring.clone()));
    let permissions = HomePermissions::capture(fixture.directory.path())?;
    read_only(fixture.directory.path())?;
    let result = delete_codex_auth();
    drop(permissions);
    outcomes.push((
        "file removal",
        true,
        result.is_err() && keychain.get_password().is_ok(),
    ));

    keychain.set_password(&serde_json::to_string(&auth("A"))?)?;
    let before = std::fs::read(&path)?;
    let permissions = HomePermissions::capture(fixture.directory.path())?;
    keyring::set_default_credential_builder(Box::new(DeleteFailure(
        fixture.keyring.clone(),
        Some(fixture.directory.path().to_owned()),
    )));
    let result = write_codex_auth(&auth("C"));
    drop(permissions);
    let error = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    let backup = std::fs::read_dir(fixture.directory.path())?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".fspec-auth-backup-")
        });
    let recoverable = backup.is_some_and(|entry| std::fs::read(entry.path()).ok() == Some(before));
    outcomes.push((
        "rollback",
        true,
        recoverable && error.contains("rollback") && error.contains("delete denied"),
    ));
    assert!(
        outcomes.iter().all(|(_, _, safe)| *safe),
        "failure safety outcomes: {outcomes:?}"
    );
    Ok(())
}
