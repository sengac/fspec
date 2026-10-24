use crate::support::{Fixture, TestResult};
use codelet_core::persistence;

impl Fixture {
    pub fn cold_restore(&self) -> TestResult {
        codelet_core::SessionManagerHandle::destroy_session(self.manager.as_ref(), &self.id)?;
        codelet_core::SessionManagerHandle::resume_session(self.manager.as_ref(), &self.id)?;
        Ok(())
    }

    pub fn seed(&self) -> TestResult {
        let mut manifest = persistence::load_session(uuid::Uuid::parse_str(&self.id.value)?)?;
        let mut content = (0..2600)
            .map(|index| serde_json::json!({"type":"text","text":format!("original-{index:04}\n")}))
            .collect::<Vec<_>>();
        content.push(serde_json::json!({"type":"tool_use","id":"call-original","name":"read_file","input":{"path":"original.txt"}}));
        persistence::append_message_with_metadata(
            &mut manifest,
            "assistant",
            "full long assistant transcript",
            std::collections::HashMap::from([(
                "message".into(),
                serde_json::json!({"role":"assistant","content":content}),
            )]),
        )?;
        persistence::append_message_with_metadata(
            &mut manifest,
            "user",
            "complete original tool result",
            std::collections::HashMap::from([(
                "message".into(),
                serde_json::json!({"role":"user","content":[{"type":"tool_result","tool_use_id":"call-original","content":"complete original tool result"}]}),
            )]),
        )?;
        Ok(())
    }
}
