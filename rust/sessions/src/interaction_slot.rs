//! Request identity and reply consumption share one lock.
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
    pub(crate) fn set(&self, value: Option<T>) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Interaction lock poisoned")?;
        *state = (uuid::Uuid::new_v4().to_string(), value);
        Ok(())
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
    pub(crate) fn respond(
        &self,
        expected: Option<&str>,
        send: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Interaction lock poisoned")?;
        if let Some(expected) = expected {
            if state.1.is_none() || state.0 != expected {
                return Err("Stale or already answered interaction".into());
            }
        }
        send()?;
        *state = (uuid::Uuid::new_v4().to_string(), None);
        Ok(())
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
            .respond(Some(&old), || {
                invoked = true;
                Ok(())
            })
            .is_err());
        assert!(!invoked);
        let (current, _) = slot.snapshot()?;
        slot.respond(Some(&current), || Ok(()))?;
        assert!(slot.respond(Some(&current), || Ok(())).is_err());
        Ok(())
    }
}
