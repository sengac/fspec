//! RPC-447: `fspec daemons add|list|remove` and saved-daemon resolution for
//! `--connect`.
//!
//! Feature: spec/features/daemons-cli.feature
//! Feature: spec/features/clients-trust-a-daemon-s-private-ca.feature

use anyhow::{Context, Result};
use codelet_common::saved_daemons::{self, SavedDaemon};
use codelet_rpc_server::DaemonCa;
use url::Url;

#[derive(clap::Subcommand, Debug)]
pub enum DaemonsAction {
    /// Save a daemon under a name (remote daemons need wss:// and --token).
    Add {
        name: String,
        url: String,
        /// Bearer token the daemon requires (FSPEC_DAEMON_TOKEN on the daemon).
        #[arg(long)]
        token: Option<String>,
        /// PEM CA certificate(s) to trust for this wss:// daemon, beside the
        /// public roots and FSPEC_DAEMON_CA (a private-CA or self-signed
        /// daemon certificate). Saved as an absolute path.
        #[arg(long)]
        ca: Option<std::path::PathBuf>,
    },
    /// List saved daemons (tokens are never printed).
    List,
    /// Remove a saved daemon by name.
    Remove { name: String },
}

pub fn run(action: DaemonsAction) -> Result<()> {
    let path = saved_daemons::default_path()?;
    match action {
        DaemonsAction::Add {
            name,
            url,
            token,
            ca,
        } => {
            let ca = ca.as_deref().map(checked_ca).transpose()?;
            saved_daemons::save(
                &path,
                SavedDaemon {
                    name: name.clone(),
                    url,
                    token,
                    ca,
                },
            )?;
            println!("✓ Saved daemon {name}");
        }
        DaemonsAction::List => {
            let daemons = saved_daemons::list(&path)?;
            if daemons.is_empty() {
                println!("No saved daemons");
            }
            for daemon in daemons {
                let token = if daemon.token.is_some() {
                    "token: set"
                } else {
                    "token: none"
                };
                match &daemon.ca {
                    Some(ca) => println!(
                        "{}  {}  ({token}, ca: {})",
                        daemon.name,
                        daemon.url,
                        ca.display()
                    ),
                    None => println!("{}  {}  ({token})", daemon.name, daemon.url),
                }
            }
        }
        DaemonsAction::Remove { name } => {
            if !saved_daemons::remove(&path, &name)? {
                anyhow::bail!("no saved daemon named {name}");
            }
            println!("✓ Removed daemon {name}");
        }
    }
    Ok(())
}

/// RPC-458: the absolute path of a `--ca` file, refused unless it holds
/// PEM certificates (the check every connect makes).
fn checked_ca(path: &std::path::Path) -> Result<std::path::PathBuf> {
    codelet_rpc_server::load_ca_file(path, &codelet_rpc_server::CaOrigin::Option)?;
    std::fs::canonicalize(path).with_context(|| format!("--ca {}", path.display()))
}

/// RPC-458: the CA a saved daemon's connects trust.
pub fn saved_ca(daemon: &SavedDaemon) -> Option<DaemonCa> {
    daemon.ca.clone().map(|path| DaemonCa {
        daemon: daemon.name.clone(),
        path,
    })
}

/// Where a `--connect` goes: the URL, the bearer to present and (RPC-458)
/// a saved daemon's own CA.
pub type ConnectTarget = (Url, Option<String>, Option<DaemonCa>);

/// Resolve a `--connect` value and the bearer to present. A `ws://` /
/// `wss://` URL uses `token` (`--token`), else `FSPEC_DAEMON_TOKEN`
/// (BUG-429); anything else names a saved daemon, which uses `token` when
/// given, else its saved token, and trusts its saved CA (RPC-458).
pub fn resolve_connect(value: &str, token: Option<String>) -> Result<ConnectTarget> {
    if value.starts_with("ws://") || value.starts_with("wss://") {
        let url = Url::parse(value).with_context(|| format!("--connect URL parse: {value}"))?;
        let bearer = match token {
            Some(token) => Some(token),
            None => crate::daemon_directory::local_daemon_token()?,
        };
        return Ok((url, bearer, None));
    }
    let daemon =
        saved_daemons::find(&saved_daemons::default_path()?, value)?.with_context(|| {
            format!("--connect: no saved daemon named {value} (see `fspec daemons list`)")
        })?;
    let url = Url::parse(&daemon.url)
        .with_context(|| format!("saved daemon {value} has an invalid URL: {}", daemon.url))?;
    let ca = saved_ca(&daemon);
    Ok((url, token.or(daemon.token), ca))
}

#[cfg(test)]
mod tests;
