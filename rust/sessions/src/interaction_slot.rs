//! Request identity and reply consumption share one lock.
//!
//! AMGR-020: every reply names the request id; there is no unchecked reply.
use std::sync::Mutex;
pub(crate) struct InteractionSlot<T> {
    state: Mutex<(String, Option<T>)>,
}
impl<T: Clone> InteractionSlot<T> {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new((String::new(), None)),
        }
    }
    /// Store `value` under a new request id; returns the request it
    /// replaced, if one was pending, with its id.
    pub(crate) fn set(&self, value: Option<T>) -> Result<Option<(String, T)>, String> {
        let mut state = self.state.lock().map_err(|_| "Interaction lock poisoned")?;
        let previous = std::mem::replace(&mut *state, (uuid::Uuid::new_v4().to_string(), value));
        Ok(previous.1.map(|value| (previous.0, value)))
    }
    pub(crate) fn snapshot(&self) -> Result<(String, Option<T>), String> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| "Interaction lock poisoned".into())
    }
    pub(crate) fn current(&self) -> Option<T> {
        self.snapshot().ok().and_then(|(_, value)| value)
    }
    /// The pending request when its id is `request_id`.
    pub(crate) fn pending_with_id(&self, request_id: &str) -> Option<T> {
        match self.snapshot() {
            Ok((id, value)) if id == request_id => value,
            _ => None,
        }
    }
    /// AMGR-021: change the pending request in place (its holder) under the
    /// slot lock, keeping its id; `None` when no request is pending.
    pub(crate) fn modify<R>(
        &self,
        change: impl FnOnce(&str, &mut T) -> Result<R, String>,
    ) -> Result<Option<R>, String> {
        let mut state = self.state.lock().map_err(|_| "Interaction lock poisoned")?;
        let (id, value) = &mut *state;
        match value {
            Some(value) => change(id, value).map(Some),
            None => Ok(None),
        }
    }
    /// Answer the pending request named `expected`: `check` may refuse it,
    /// then `send` delivers the answer. Returns the answered request.
    pub(crate) fn respond(
        &self,
        expected: &str,
        check: impl FnOnce(&T) -> Result<(), String>,
        send: impl FnOnce() -> Result<(), String>,
    ) -> Result<T, String> {
        let mut state = self.state.lock().map_err(|_| "Interaction lock poisoned")?;
        let pending = match &state.1 {
            Some(value) if state.0 == expected => value.clone(),
            _ => return Err("Stale or already answered interaction".into()),
        };
        check(&pending)?;
        send()?;
        *state = (uuid::Uuid::new_v4().to_string(), None);
        Ok(pending)
    }
    /// BUG-605: answer the pending request with `send`; nothing is sent
    /// when no request is pending, so no stale answer is left behind.
    /// Returns the cancelled request with its id.
    pub(crate) fn cancel_pending(
        &self,
        send: impl FnOnce() -> Result<(), String>,
    ) -> Result<Option<(String, T)>, String> {
        let mut state = self.state.lock().map_err(|_| "Interaction lock poisoned")?;
        let Some(pending) = state.1.clone() else {
            return Ok(None);
        };
        send()?;
        let id = std::mem::replace(&mut *state, (uuid::Uuid::new_v4().to_string(), None)).0;
        Ok(Some((id, pending)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_replacement_and_duplicate_replies_are_rejected() -> Result<(), String> {
        let slot = InteractionSlot::new();
        slot.set(Some("question"))?;
        let (old, _) = slot.snapshot()?;
        slot.set(Some("question"))?;
        let mut invoked = false;
        assert!(slot
            .respond(
                &old,
                |_| Ok(()),
                || {
                    invoked = true;
                    Ok(())
                }
            )
            .is_err());
        assert!(!invoked);
        let (current, _) = slot.snapshot()?;
        slot.respond(&current, |_| Ok(()), || Ok(()))?;
        assert!(slot.respond(&current, |_| Ok(()), || Ok(())).is_err());
        Ok(())
    }
}
