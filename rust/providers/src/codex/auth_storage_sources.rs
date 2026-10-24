use super::Result;
#[cfg(target_os = "macos")]
use super::{AuthError, CodexAuthJson};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn file_document(home: &Path) -> Result<Value> {
    match fs::read(home.join("auth.json")) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(error.into()),
    }
}

#[cfg(target_os = "macos")]
fn preferred(home: &Path) -> Result<Option<Value>> {
    match super::keychain(home)?.get_password() {
        Ok(password) => {
            let document: Value = serde_json::from_str(&password)?;
            if !document.is_object() {
                return Err(AuthError::Changed);
            }
            serde_json::from_value::<CodexAuthJson>(document.clone())?;
            Ok(Some(document))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn document(home: &Path) -> Result<Value> {
    #[cfg(not(target_os = "macos"))]
    return file_document(home);
    #[cfg(target_os = "macos")]
    {
        let Some(preferred) = preferred(home)? else {
            return file_document(home);
        };
        let mut document = match file_document(home) {
            Ok(document) if document.is_object() => document,
            Ok(_) | Err(AuthError::Json(_)) => serde_json::json!({}),
            Err(error) => return Err(error),
        };
        let same_tokens = preferred
            .get("tokens")
            .is_some_and(|tokens| !tokens.is_null() && document.get("tokens") == Some(tokens));
        let fields = document.as_object_mut().ok_or(AuthError::Changed)?;
        for field in ["tokens", "last_refresh"] {
            fields.remove(field);
        }
        if !same_tokens {
            for field in ["fspec_generation", "fspec_expires_at"] {
                fields.remove(field);
            }
        }
        if fields
            .get("OPENAI_API_KEY")
            .is_some_and(|key| !key.is_string() && !key.is_null())
        {
            fields.remove("OPENAI_API_KEY");
        }
        for (name, value) in preferred.as_object().ok_or(AuthError::Changed)? {
            if name != "OPENAI_API_KEY" || !value.is_null() {
                fields.insert(name.clone(), value.clone());
            }
        }
        Ok(document)
    }
}
