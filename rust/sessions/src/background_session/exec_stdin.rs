use super::*;

impl BackgroundSession {
    pub fn set_exec_stdin_request(
        &self,
        mut request: Option<codelet_tools::unified_exec::ExecStdinRequest>,
    ) {
        let alive = match &request {
            Some(request) => self.exec_session_alive(&request.exec_session_id),
            None => true,
        };
        if !alive {
            request = None;
        }
        if let Ok(mut guard) = self.exec_stdin_request.write() {
            let prev_same_fire = match (&*guard, &request) {
                (Some(prev), Some(next)) => {
                    prev.exec_session_id == next.exec_session_id && prev.ts_ms == next.ts_ms
                }
                (None, None) => true,
                _ => false,
            };
            let chunk = match request.as_ref() {
                Some(internal) => Some(StreamChunk::exec_stdin_request(
                    crate::exec_stdin_mapping::internal_request_to_wire(internal.clone()),
                )),
                None => Some(StreamChunk::exec_stdin_request_cleared()),
            };
            *guard = request;
            if !prev_same_fire {
                if let Some(chunk) = chunk {
                    self.handle_output(chunk);
                }
            }
        }
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
                matches!(store.try_wait(exec_session_id).await, Some(None))
            })
        })
    }

    pub fn get_exec_stdin_request(&self) -> Option<codelet_tools::unified_exec::ExecStdinRequest> {
        let request = self.exec_stdin_request.read().ok()?.clone()?;
        if !self.exec_session_alive(&request.exec_session_id) {
            self.set_exec_stdin_request(None);
            return None;
        }
        Some(request)
    }
}
