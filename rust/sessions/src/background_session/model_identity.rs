//! BUG-464: the session's selected provider and model ids, read without
//! substituting an empty string for a poisoned lock or an unset id.

use super::BackgroundSession;

impl BackgroundSession {
    /// The selected `(provider_id, model_id)`, or an error naming the session
    /// when either lock is poisoned or either id is unset.
    pub(crate) fn selected_model_ids(&self) -> Result<(String, String), String> {
        let read = |slot: &std::sync::RwLock<Option<String>>, what: &str| {
            slot.read()
                .map_err(|_| format!("the {what} lock of session {} is poisoned", self.id))?
                .clone()
                .ok_or_else(|| format!("session {} has no {what} selected", self.id))
        };
        Ok((
            read(&self.provider_id, "provider")?,
            read(&self.model_id, "model")?,
        ))
    }
}
