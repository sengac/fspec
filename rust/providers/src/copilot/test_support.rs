use super::{
    get_copilot_auth_path,
    token_exchange::{exchange_with_client, TokenExchangeResponse},
};
use crate::ProviderError;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use url::{Host, Url};

#[derive(Clone)]
struct Issuer {
    identity: uuid::Uuid,
    root: PathBuf,
    url: String,
}

fn canonical_destination(path: &Path) -> Result<PathBuf, ProviderError> {
    let mut ancestor = path;
    let mut remaining = Vec::new();
    loop {
        match ancestor.symlink_metadata() {
            Ok(_) => {
                let mut destination = ancestor
                    .canonicalize()
                    .map_err(|_| invalid("fixture path cannot be resolved"))?;
                for component in remaining.into_iter().rev() {
                    destination.push(component);
                }
                return Ok(destination);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                remaining.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| invalid("invalid fixture path"))?,
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| invalid("invalid fixture ancestor"))?;
            }
            Err(_) => return Err(invalid("fixture path cannot be inspected")),
        }
    }
}

static ISSUER: Mutex<Option<Issuer>> = Mutex::new(None);

fn invalid(message: &str) -> ProviderError {
    ProviderError::config("copilot synthetic issuer", message)
}

pub(crate) fn validate_url(value: &str) -> Result<(), ProviderError> {
    let url = Url::parse(value).map_err(|_| invalid("invalid fixture URL"))?;
    let local = match url.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    };
    if !local
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid(
            "fixture requires a numeric-loopback URL without userinfo or fragment",
        ));
    }
    Ok(())
}

pub struct TestIssuerGuard {
    identity: uuid::Uuid,
}

impl TestIssuerGuard {
    pub fn install(root: &Path, url: &str) -> Result<Self, ProviderError> {
        validate_url(url)?;
        let root = root
            .canonicalize()
            .map_err(|_| invalid("fixture root missing"))?;
        let temporary = std::env::temp_dir()
            .canonicalize()
            .map_err(|_| invalid("temporary directory unavailable"))?;
        if !root.starts_with(temporary)
            || canonical_destination(&get_copilot_auth_path())?
                != root.join(".fspec/credentials/copilot_auth.json")
        {
            return Err(invalid(
                "fixture credentials are not isolated in the temporary root",
            ));
        }
        let mut selected = ISSUER
            .lock()
            .map_err(|_| invalid("fixture issuer lock poisoned"))?;
        if selected.is_some() {
            return Err(invalid("a fixture issuer is already active"));
        }
        let identity = uuid::Uuid::new_v4();
        *selected = Some(Issuer {
            identity,
            root,
            url: url.into(),
        });
        Ok(Self { identity })
    }

    pub async fn probe(
        &self,
        synthetic_token: &str,
    ) -> Result<TokenExchangeResponse, ProviderError> {
        let selected = active()?
            .filter(|issuer| issuer.identity == self.identity)
            .ok_or_else(|| invalid("fixture issuer is not active"))?;
        probe(&selected, synthetic_token).await
    }
}

impl Drop for TestIssuerGuard {
    fn drop(&mut self) {
        if let Ok(mut selected) = ISSUER.lock() {
            if selected
                .as_ref()
                .is_some_and(|issuer| issuer.identity == self.identity)
            {
                *selected = None;
            }
        }
    }
}

fn active() -> Result<Option<Issuer>, ProviderError> {
    let selected = ISSUER
        .lock()
        .map_err(|_| invalid("fixture issuer lock poisoned"))?
        .clone();
    if let Some(issuer) = &selected {
        if canonical_destination(&get_copilot_auth_path())?
            != issuer.root.join(".fspec/credentials/copilot_auth.json")
        {
            return Err(invalid("credential root changed during fixture scope"));
        }
    }
    Ok(selected)
}

async fn probe(issuer: &Issuer, token: &str) -> Result<TokenExchangeResponse, ProviderError> {
    if token.is_empty() {
        return Err(invalid("synthetic token must not be empty"));
    }
    validate_url(&issuer.url)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| invalid("failed to construct isolated fixture client"))?;
    let response = exchange_with_client(&issuer.url, token, client).await?;
    if !response.endpoints_api.is_empty() {
        validate_url(&response.endpoints_api)?;
    }
    Ok(response)
}

pub(crate) async fn exchange_if_active(
    token: &str,
) -> Option<Result<TokenExchangeResponse, ProviderError>> {
    match active() {
        Ok(Some(issuer)) => Some(probe(&issuer, token).await),
        Ok(None) => None,
        Err(error) => Some(Err(error)),
    }
}
