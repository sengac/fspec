use super::*;

impl BackgroundSession {
    /// Record (or clear) the session's pending exec-stdin request and push
    /// the matching chunk. BUG-369: a poisoned state lock is an error.
    pub fn set_exec_stdin_request(
        &self,
        mut request: Option<codelet_tools::unified_exec::ExecStdinRequest>,
    ) -> Result<(), String> {
        let alive = match &request {
            Some(request) => self.exec_session_alive(&request.exec_session_id),
            None => true,
        };
        if !alive {
            request = None;
        }
        let chunk = match request.as_ref() {
            Some(internal) => StreamChunk::exec_stdin_request(
                crate::exec_stdin_mapping::internal_request_to_wire(internal.clone()),
            ),
            None => StreamChunk::exec_stdin_request_cleared(),
        };
        // BUG-648: the change is journaled while the slot is still locked,
        // so the journal records slot changes in the order they took effect.
        crate::lock_state::write_state(&self.exec_stdin_request, "exec-stdin request", |slot| {
            let same = match (&*slot, &request) {
                (Some(prev), Some(next)) => {
                    prev.exec_session_id == next.exec_session_id && prev.ts_ms == next.ts_ms
                }
                (None, None) => true,
                _ => false,
            };
            *slot = request;
            if !same {
                self.handle_output(chunk);
            }
        })?;
        Ok(())
    }

    fn exec_session_alive(&self, exec_session_id: &str) -> bool {
        use codelet_tools::unified_exec::global_store;
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return true,
        };
        if !matches!(
            handle.runtime_flavor(),
            tokio::runtime::RuntimeFlavor::MultiThread
        ) {
            return true;
        }
        let store = global_store();
        tokio::task::block_in_place(|| {
            handle.block_on(async move {
                if !store.contains(exec_session_id).await {
                    return false;
                }
                match store.try_wait(exec_session_id).await {
                    Ok(state) => matches!(state, Some(None)),
                    Err(error) => {
                        // BUG-363: report it; an unchecked session is not live.
                        tracing::error!(session = exec_session_id, %error, "cannot check exec session");
                        false
                    }
                }
            })
        })
    }

    /// The pending exec-stdin request, if its exec session is still live.
    /// BUG-369: a poisoned state lock is an error, not "no request".
    pub fn get_exec_stdin_request(
        &self,
    ) -> Result<Option<codelet_tools::unified_exec::ExecStdinRequest>, String> {
        let Some(request) =
            crate::lock_state::read_state(&self.exec_stdin_request, "exec-stdin request")?
        else {
            return Ok(None);
        };
        if !self.exec_session_alive(&request.exec_session_id) {
            self.set_exec_stdin_request(None)?;
            return Ok(None);
        }
        Ok(Some(request))
    }
}
