use super::fixture::{history, EffectTool, Harness, TestResult, MODEL};
use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::run_agent_stream;
use codelet_core::RigAgent;
use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use serde_json::json;
use std::time::Duration;

impl Harness {
    pub async fn fresh(&self) -> TestResult {
        let server = crate::transport::Server::start(vec![crate::transport::event(json!({
            "type":"response.completed","response":{
                "id":"fresh-response","status":"completed","end_turn":true,
                "output":[{"type":"message","id":"fresh-answer","role":"assistant",
                    "content":[{"type":"output_text","text":"fresh instruction completed","annotations":[]}]}],
                "usage":{"input_tokens":30,"output_tokens":5,"total_tokens":35}
            }}))], false).await?;
        self.session.reset_interrupt();
        let _admission = codelet_tools::tool_admission::register_auxiliary(
            self.session.id,
            self.directory.path().into(),
        )?;
        let mut inner = self.session.inner.lock().await;
        let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
            .api_key("synthetic-offline-key")
            .base_url(&server.url)
            .http_client(RefreshingCodexClient::new_api_key().with_test_endpoint(&server.url)?)
            .build()?;
        let model = CodexResponsesModel::new(client, MODEL)
            .with_history(history(&inner)?)
            .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            });
        let agent = rig::agent::AgentBuilder::new(model)
            .tool(EffectTool(self.directory.path().join("effect")))
            .build();
        codelet_agent_loop::persist::persist_user_message(&self.session.id, "fresh instruction")?;
        tokio::time::timeout(
            Duration::from_secs(15),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                "fresh instruction",
                &mut inner,
                self.session.is_interrupted.clone(),
                self.session.compaction_in_progress.clone(),
                self.session.interrupt_notify.clone(),
                &BackgroundOutput::with_provider(self.session.clone(), "codex".into(), None),
                self.session.id,
            ),
        )
        .await??;
        let requests = server.requests.lock().map_err(|_| "fresh requests")?;
        assert_eq!(requests.len(), 1);
        let wire = serde_json::to_string(&requests[0])?;
        for canceled in [
            "six-case-",
            "two-case-",
            "hook cancellation",
            "persistence cancellation",
        ] {
            assert!(
                !wire.contains(canceled),
                "canceled input reached the fresh provider request"
            );
        }
        assert!(wire.contains("fresh instruction"));
        assert_eq!(
            std::fs::read_to_string(self.directory.path().join("effect"))?,
            "executed once"
        );
        Ok(())
    }
}
