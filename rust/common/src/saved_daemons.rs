//! RPC-447: saved remote daemons.
//!
//! Feature: spec/features/saved-remote-daemons.feature
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! `~/.fspec/daemons.json` lists the daemons a user saved by name, with the
//! bearer token clients send on connect. The file is owner-only (0600) since
//! it holds tokens. A remote (non-loopback) daemon must be reached over
//! `wss://` with a token; plain `ws://` is accepted only for loopback hosts.
//!
//! TUI-123: names and URLs are validated and normalised here for both
//! `fspec daemons add` and the TUI (see [`validate_name`] and
//! [`normalize_url`]). Every write holds BUG-477's cross-process
//! [`SpecLock`] on the file's directory across the read-modify-write and
//! replaces the file through a uniquely named owner-only temp file, so
//! concurrent writers never lose or corrupt entries. A malformed or
//! unreadable file is an error naming its path and is never overwritten.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::spec_lock::SpecLock;

const VERSION: u32 = 1;

/// TUI-123: the longest daemon name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedDaemon {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// RPC-458: the absolute path of a PEM CA file trusted (beside the
    /// public roots) when connecting to this daemon (`fspec daemons add
    /// --ca`). Only a `wss://` daemon has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca: Option<PathBuf>,
}

/// TUI-121: never print a saved token.
impl std::fmt::Debug for SavedDaemon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SavedDaemon")
            .field("name", &self.name)
            .field("url", &self.url)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("ca", &self.ca)
            .finish()
    }
}

/// Which form field a refusal concerns (TUI-123: the TUI shows the reason
/// under that field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonField {
    Name,
    Url,
    Token,
}

#[derive(Debug, Error)]
pub enum SavedDaemonError {
    #[error("a daemon name is required")]
    EmptyName,
    /// TUI-123: a name with a control character (a pasted tab, escape...).
    #[error("a daemon name must not contain a control character")]
    NameControlCharacter,
    /// TUI-123: a name longer than [`MAX_NAME_CHARS`].
    #[error("a daemon name must be at most {MAX_NAME_CHARS} characters (this one has {0})")]
    NameTooLong(usize),
    /// TUI-123: another saved daemon has this name, ignoring case.
    #[error("a daemon named {0} is already saved")]
    NameTaken(String),
    #[error("remote daemons must use wss:// (plain ws:// is only allowed for loopback): {0}")]
    InsecureRemote(String),
    #[error("remote daemon {0} needs a token (--token)")]
    MissingToken(String),
    /// RPC-458: a CA is a TLS setting; a plain `ws://` daemon has none.
    #[error("--ca applies only to a wss:// daemon: {0}")]
    CaWithoutTls(String),
    #[error("daemon URL must start with ws:// or wss://: {0}")]
    UnsupportedScheme(String),
    /// TUI-123: no `scheme://` at all (the URL is not repeated: it may
    /// hold a password).
    #[error("daemon URL needs a scheme: wss://host[:port] (or ws:// for this machine)")]
    MissingScheme,
    #[error("daemon URL has no host: {0}")]
    EmptyHost(String),
    /// TUI-123: the URL is never repeated — it holds a password.
    #[error(
        "daemon URL must not contain a user name or password; put the bearer token in the token field"
    )]
    CredentialsInUrl,
    #[error("daemon URL must not have a query or fragment: {0}")]
    QueryOrFragment(String),
    #[error("daemon URL has an invalid port (use 1-65535): {0}")]
    InvalidPort(String),
    #[error("daemon URL has an invalid host: {0}")]
    InvalidHost(String),
    #[error("cannot locate the home directory")]
    NoHome,
    /// TUI-121: editing a daemon that is not saved.
    #[error("no saved daemon named {0}")]
    NotSaved(String),
    /// TUI-123: the file exists but cannot be read; it is left alone.
    #[error("{path} could not be read ({source}); it was not changed")]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// TUI-123: the file is not saved-daemons JSON; it is left alone.
    #[error(
        "{path} is not valid saved-daemons JSON ({reason}); fix or remove it, it was not changed"
    )]
    Malformed { path: PathBuf, reason: String },
    #[error("saved daemons I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("saved daemons file is malformed: {0}")]
    Json(#[from] serde_json::Error),
}

impl SavedDaemonError {
    /// TUI-123: the form field this refusal concerns (`None`: the file).
    pub fn field(&self) -> Option<DaemonField> {
        match self {
            Self::EmptyName
            | Self::NameControlCharacter
            | Self::NameTooLong(_)
            | Self::NameTaken(_)
            | Self::NotSaved(_) => Some(DaemonField::Name),
            Self::InsecureRemote(_)
            | Self::UnsupportedScheme(_)
            | Self::MissingScheme
            | Self::EmptyHost(_)
            | Self::CredentialsInUrl
            | Self::QueryOrFragment(_)
            | Self::InvalidPort(_)
            | Self::InvalidHost(_) => Some(DaemonField::Url),
            Self::MissingToken(_) => Some(DaemonField::Token),
            Self::CaWithoutTls(_) => Some(DaemonField::Url),
            Self::NoHome
            | Self::Unreadable { .. }
            | Self::Malformed { .. }
            | Self::Io(_)
            | Self::Json(_) => None,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct SavedDaemonsFile {
    version: u32,
    daemons: Vec<SavedDaemon>,
}

/// `~/.fspec/daemons.json`.
pub fn default_path() -> Result<PathBuf, SavedDaemonError> {
    let home = std::env::var_os("HOME").ok_or(SavedDaemonError::NoHome)?;
    Ok(PathBuf::from(home).join(".fspec").join("daemons.json"))
}

/// TUI-123: the trimmed `name`, refused when empty, holding a control
/// character or longer than [`MAX_NAME_CHARS`].
pub fn validate_name(name: &str) -> Result<String, SavedDaemonError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(SavedDaemonError::EmptyName);
    }
    if name.chars().any(char::is_control) {
        return Err(SavedDaemonError::NameControlCharacter);
    }
    let length = name.chars().count();
    if length > MAX_NAME_CHARS {
        return Err(SavedDaemonError::NameTooLong(length));
    }
    Ok(name.to_owned())
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|ip| ip.is_loopback())
        || host
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// The host of `authority` (brackets kept off IPv6) and its port text.
fn split_authority(authority: &str) -> Result<(&str, Option<&str>), SavedDaemonError> {
    if let Some(v6) = authority.strip_prefix('[') {
        let (host, after) = v6
            .split_once(']')
            .ok_or_else(|| SavedDaemonError::InvalidHost(authority.to_owned()))?;
        if host.is_empty()
            || !host
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
        {
            return Err(SavedDaemonError::InvalidHost(authority.to_owned()));
        }
        return match after {
            "" => Ok((host, None)),
            rest => rest
                .strip_prefix(':')
                .map(|port| (host, Some(port)))
                .ok_or_else(|| SavedDaemonError::InvalidHost(authority.to_owned())),
        };
    }
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if !host.is_empty()
        && !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
    {
        return Err(SavedDaemonError::InvalidHost(authority.to_owned()));
    }
    Ok((host, port))
}

/// TUI-123: validate a daemon URL and return its normal form —
/// `ws://` or `wss://` with a host; no credentials, query or fragment; a
/// port of 1-65535 when given (kept as typed); a trailing slash dropped.
/// `ws://` is allowed only for loopback hosts (localhost, 127.0.0.0/8,
/// ::1) and a non-loopback `wss://` daemon needs a token.
pub fn normalize_url(url: &str, token: Option<&str>) -> Result<String, SavedDaemonError> {
    let url = url.trim();
    let Some((scheme, rest)) = url.split_once("://") else {
        if url
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .contains('@')
        {
            return Err(SavedDaemonError::CredentialsInUrl);
        }
        return Err(SavedDaemonError::MissingScheme);
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    if authority.contains('@') {
        return Err(SavedDaemonError::CredentialsInUrl);
    }
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "ws" && scheme != "wss" {
        return Err(SavedDaemonError::UnsupportedScheme(url.to_owned()));
    }
    if tail.contains(['?', '#']) {
        return Err(SavedDaemonError::QueryOrFragment(url.to_owned()));
    }
    if tail.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(SavedDaemonError::InvalidHost(url.to_owned()));
    }
    let (host, port) = split_authority(authority)?;
    if host.is_empty() {
        return Err(SavedDaemonError::EmptyHost(url.to_owned()));
    }
    if let Some(port) = port {
        let valid = !port.is_empty()
            && port.chars().all(|c| c.is_ascii_digit())
            && port.parse::<u16>().is_ok_and(|port| port > 0);
        if !valid {
            return Err(SavedDaemonError::InvalidPort(url.to_owned()));
        }
    }
    let normalized = format!("{scheme}://{authority}{}", tail.trim_end_matches('/'));
    let has_token = token.is_some_and(|t| !t.trim().is_empty());
    match (scheme.as_str(), is_loopback(host)) {
        ("ws", false) => Err(SavedDaemonError::InsecureRemote(normalized)),
        ("wss", false) if !has_token => Err(SavedDaemonError::MissingToken(normalized)),
        _ => Ok(normalized),
    }
}

/// Reject insecure remote URLs and token-less `wss://` daemons (and,
/// since TUI-123, every URL [`normalize_url`] refuses).
pub fn validate(url: &str, token: Option<&str>) -> Result<(), SavedDaemonError> {
    normalize_url(url, token).map(|_| ())
}

/// Every saved daemon (empty when the file does not exist yet). TUI-123:
/// an unreadable or malformed file is an error naming the path.
pub fn list(path: &Path) -> Result<Vec<SavedDaemon>, SavedDaemonError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str::<SavedDaemonsFile>(&text)
            .map(|file| file.daemons)
            .map_err(|error| SavedDaemonError::Malformed {
                path: path.to_owned(),
                reason: error.to_string(),
            }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(SavedDaemonError::Unreadable {
            path: path.to_owned(),
            source,
        }),
    }
}

/// The saved daemon called `name`.
pub fn find(path: &Path, name: &str) -> Result<Option<SavedDaemon>, SavedDaemonError> {
    Ok(list(path)?.into_iter().find(|daemon| daemon.name == name))
}

/// Run `change` on the saved daemons under the cross-process lock and
/// write the result; `change` returning `None` writes nothing.
fn modify<T>(
    path: &Path,
    change: impl FnOnce(&mut Vec<SavedDaemon>) -> Result<Option<T>, SavedDaemonError>,
) -> Result<Option<T>, SavedDaemonError> {
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    std::fs::create_dir_all(&dir)?;
    let _lock = SpecLock::acquire(&dir)?;
    let mut daemons = list(path)?;
    let outcome = change(&mut daemons)?;
    if outcome.is_some() {
        write(path, &dir, daemons)?;
    }
    Ok(outcome)
}

/// Validate and save `daemon`, replacing any saved daemon with its name.
pub fn save(path: &Path, daemon: SavedDaemon) -> Result<(), SavedDaemonError> {
    let name = validate_name(&daemon.name)?;
    let url = normalize_url(&daemon.url, daemon.token.as_deref())?;
    if daemon.ca.is_some() && !url.starts_with("wss://") {
        return Err(SavedDaemonError::CaWithoutTls(url));
    }
    let daemon = SavedDaemon {
        name,
        url,
        token: daemon.token,
        ca: daemon.ca,
    };
    modify(path, |daemons| {
        daemons.retain(|known| known.name != daemon.name);
        daemons.push(daemon);
        Ok(Some(()))
    })?;
    Ok(())
}

/// TUI-121: the token an add or edit saves. `Keep` keeps the edited
/// daemon's saved token (no token when adding); `Debug` never shows a token.
#[derive(Clone, PartialEq, Eq)]
pub enum TokenChange {
    Keep,
    Set(String),
}

impl std::fmt::Debug for TokenChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenChange::Keep => f.write_str("Keep"),
            TokenChange::Set(_) => f.write_str("Set(<redacted>)"),
        }
    }
}

fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// TUI-121: save `name` at `url`, replacing `previous` (the edited daemon,
/// `None` when adding). A rename removes the previous entry. The usual
/// validation applies to the resulting URL and token. TUI-123: a name
/// another saved daemon already has (ignoring case) is refused, and the
/// whole change is one locked write.
pub fn update(
    path: &Path,
    previous: Option<&str>,
    name: &str,
    url: &str,
    token: TokenChange,
) -> Result<SavedDaemon, SavedDaemonError> {
    let name = validate_name(name)?;
    let saved = modify(path, |daemons| {
        let token = match (token, previous) {
            (TokenChange::Set(token), _) => Some(token),
            (TokenChange::Keep, None) => None,
            (TokenChange::Keep, Some(previous)) => daemons
                .iter()
                .find(|daemon| daemon.name == previous)
                .ok_or_else(|| SavedDaemonError::NotSaved(previous.to_owned()))?
                .token
                .clone(),
        };
        let url = normalize_url(url, token.as_deref())?;
        if let Some(clash) = daemons
            .iter()
            .find(|daemon| Some(daemon.name.as_str()) != previous && same_name(&daemon.name, &name))
        {
            return Err(SavedDaemonError::NameTaken(clash.name.clone()));
        }
        // RPC-458: an edit keeps the edited daemon's CA (set only by
        // `fspec daemons add --ca`); it is dropped if the URL is no longer
        // wss://.
        let ca = previous
            .and_then(|previous| daemons.iter().find(|daemon| daemon.name == previous))
            .and_then(|daemon| daemon.ca.clone())
            .filter(|_| url.starts_with("wss://"));
        let daemon = SavedDaemon {
            name: name.clone(),
            url,
            token,
            ca,
        };
        match previous.and_then(|previous| daemons.iter().position(|d| d.name == previous)) {
            Some(index) => daemons[index] = daemon.clone(),
            None => daemons.push(daemon.clone()),
        }
        Ok(Some(daemon))
    })?;
    saved.ok_or(SavedDaemonError::NotSaved(name))
}

/// Remove the saved daemon called `name`; `false` when none was saved.
pub fn remove(path: &Path, name: &str) -> Result<bool, SavedDaemonError> {
    let removed = modify(path, |daemons| {
        let before = daemons.len();
        daemons.retain(|daemon| daemon.name != name);
        Ok((daemons.len() != before).then_some(()))
    })?;
    Ok(removed.is_some())
}

/// Replace `path` with `daemons`: a uniquely named owner-only temp file in
/// `dir`, fsynced, renamed over `path`, then the directory fsynced. A
/// failed write removes its temp file and leaves `path` as it was.
fn write(path: &Path, dir: &Path, daemons: Vec<SavedDaemon>) -> Result<(), SavedDaemonError> {
    let body = serde_json::to_vec_pretty(&SavedDaemonsFile {
        version: VERSION,
        daemons,
    })?;
    let staged = dir.join(format!(".daemons.json.tmp.{}", uuid::Uuid::new_v4()));
    let written = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        // The file holds bearer tokens: create it owner-only from the start.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&staged)?;
        std::io::Write::write_all(&mut file, &body)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&staged, path)
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_file(&staged);
        return Err(error.into());
    }
    sync_dir(dir)?;
    Ok(())
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}
