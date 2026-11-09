macro_rules! compose_handle_impl_10 {
    ($($collected:tt)*) => {
        compose_handle_impl_finish! {
            $($collected)*
    fn loop_cancel(&self, id: &str) -> Result<bool, String> {
        let id_owned = id.to_string();
        let removed = loop_block_on(async move {
            codelet_core::loops::LoopStore::instance()
                .cancel(&id_owned)
                .await
        });
        Ok(removed)
    }
    fn loop_list(&self, session_id: &SessionId) -> Result<Vec<RegisteredLoop>, String> {
        // BUG-465: an unknown session is the lookup error, not "no loops".
        let session_uuid = self.get_session(session_id.value.as_str())?.id;
        let entries = loop_block_on(async move {
            codelet_core::loops::LoopStore::instance()
                .list_for_session(session_uuid)
                .await
        });
        Ok(entries.iter().map(entry_to_wire).collect())
    }
        }
    };
}
