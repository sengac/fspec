use super::{
    custom_oauth::custom_oauth_store_path, script_provider::ScriptedOAuthProvider,
    scripted_refreshing_client::ScriptedRefreshingClient,
};
use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
};

type Gate = tokio::sync::Mutex<()>;
type Gates = Mutex<HashMap<PathBuf, Weak<Gate>>>;
static GATES: OnceLock<Gates> = OnceLock::new();

pub(crate) fn credential_path(name: &str) -> Result<PathBuf> {
    let path = custom_oauth_store_path(name);
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut ancestor = path.as_path();
    let mut remaining = Vec::new();
    loop {
        match ancestor.symlink_metadata() {
            Ok(_) => {
                let mut resolved = ancestor.canonicalize()?;
                for component in remaining.into_iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                remaining.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| anyhow!("invalid credential path"))?,
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| anyhow!("invalid credential ancestor"))?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

pub(crate) fn check_root(name: &str, expected: &Path) -> Result<()> {
    if credential_path(name)? != expected {
        return Err(anyhow!("scripted OAuth credential root changed"));
    }
    Ok(())
}

fn gate(path: &Path) -> Result<Arc<Gate>> {
    let mut gates = GATES
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|_| anyhow!("scripted OAuth coordination unavailable"))?;
    gates.retain(|_, gate| gate.strong_count() > 0);
    if let Some(gate) = gates.get(path).and_then(Weak::upgrade) {
        return Ok(gate);
    }
    let gate = Arc::new(Gate::default());
    gates.insert(path.to_owned(), Arc::downgrade(&gate));
    Ok(gate)
}

pub(crate) async fn ensure(provider: ScriptedOAuthProvider) -> Result<()> {
    let name = provider.config().name.clone();
    let path = credential_path(&name)?;
    let gate = gate(&path)?;
    let (alive, cancelled) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let _guard = tokio::select! {
            biased;
            _ = cancelled => return Ok(()),
            guard = gate.lock() => guard,
        };
        ScriptedRefreshingClient::new(&provider, &name)
            .ensure_fresh_at(&path)
            .await?;
        check_root(&name, &path)
    });
    let outcome = task.await;
    drop(alive);
    outcome.map_err(|_| anyhow!("scripted OAuth refresh task unavailable"))?
}
