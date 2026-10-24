use codelet_cli::session::Session;
use codelet_core::persistence::{
    load_session, update_session_manifest, ManifestCommitError, SessionManifest,
};
use uuid::Uuid;

pub(crate) struct SelectionEnvironment {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl SelectionEnvironment {
    pub(crate) fn capture(provider: &str) -> Self {
        let mut keys = Vec::new();
        if provider.contains(':') {
            keys.extend([
                "OPENAI_BASE_URL",
                "OPENAI_API_KEY",
                "OPENAI_CONTEXT_WINDOW",
                "OPENAI_STREAMING",
            ]);
        } else if codelet_providers::custom_provider_registered(provider) {
            keys.extend([
                "OPENAI_BASE_URL",
                "OPENAI_MODEL",
                "OPENAI_API_KEY",
                "ANTHROPIC_API_KEY",
            ]);
        }
        if let Some(key) = crate::credentials::get_primary_env_var(provider) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        if provider == "anthropic" {
            keys.push("CLAUDE_CODE_OAUTH_TOKEN");
        }
        Self {
            saved: keys
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect(),
        }
    }

    pub(crate) fn commit(mut self) {
        self.saved.clear();
    }
}

impl Drop for SelectionEnvironment {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn validate_identity(manifest: &SessionManifest, model: &str) -> Result<(), ManifestCommitError> {
    if manifest.compaction_commit.is_some() && manifest.provider != model {
        return Err(ManifestCommitError::Ownership {
            session_id: manifest.id,
            reason: "cannot switch the selected identity of a committed compaction generation"
                .into(),
        });
    }
    Ok(())
}

pub(crate) fn validate_selection(session_id: Uuid, model: &str) -> Result<(), String> {
    validate_identity(&load_session(session_id)?, model).map_err(|error| error.to_string())
}

pub(crate) fn persist_selection(
    session: &mut Session,
    session_id: Uuid,
    model: &str,
) -> Result<(), String> {
    let manifest = load_session(session_id)?;
    validate_identity(&manifest, model).map_err(|error| error.to_string())?;
    if manifest.compaction_commit.is_some() {
        return Ok(());
    }
    session
        .reset_provider_state()
        .map_err(|error| error.to_string())?;
    match update_session_manifest(session_id, |current| {
        validate_identity(current, model)?;
        current.provider = model.to_owned();
        Ok(())
    }) {
        Ok(_) => {}
        Err(error @ ManifestCommitError::Promoted { .. }) => {
            tracing::warn!(%session_id, %error, "adopting published model selection without durability acknowledgement");
        }
        Err(error) => return Err(error.to_string()),
    }
    Ok(())
}
