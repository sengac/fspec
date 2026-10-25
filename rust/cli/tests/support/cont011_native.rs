use super::{
    runtime,
    wire::{self, Kind, Scenario},
};
use codelet_cli::{
    interactive::{run_agent_stream, ContinueStateReason, StreamEvent, StreamOutput},
    session::Session,
};
use codelet_core::RigAgent;
use codelet_providers::ProviderType;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::wasm_compat::WasmCompatSend;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
pub struct Output {
    events: Mutex<Vec<StreamEvent>>,
    interrupted: Arc<AtomicBool>,
    cancel: bool,
}
impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        if self.cancel && matches!(&event, StreamEvent::Text(text) if text == "partial native") {
            self.interrupted.store(true, Ordering::Release);
        }
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}
impl Output {
    pub fn events(&self) -> Vec<StreamEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

pub struct Case {
    pub session: Session,
    pub session_id: uuid::Uuid,
    pub output: Output,
    pub effect: PathBuf,
    pub rejections: Arc<Mutex<Vec<u32>>>,
    pub(super) kind: Kind,
    pub(super) server: wiremock::MockServer,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
    security: wiremock::MockServer,
    _runtime: runtime::RuntimeFixture,
}

impl Drop for Case {
    fn drop(&mut self) {
        codelet_tools::set_continue_armed(self.session_id, false);
        codelet_tools::set_session_goal(self.session_id, None);
    }
}

impl Case {
    pub async fn new(kind: Kind, scenario: Scenario) -> TestResult<Self> {
        let fixture = runtime::RuntimeFixture::new()?;
        let provider = match kind {
            Kind::Claude | Kind::OAuth => ProviderType::Claude,
            Kind::Gemini => ProviderType::Gemini,
            Kind::Zai => ProviderType::ZAI,
        };
        let mut manager = fixture.manager(
            provider.clone(),
            serde_json::json!({"maxOutputTokens":8192}),
        )?;
        manager.override_model_limits(None, Some(8192));
        let session_id = fixture.session_id(provider.as_str())?;
        let root = codelet_common::get_data_dir()?;
        let admission =
            codelet_tools::tool_admission::register_auxiliary(session_id, root.clone())?;
        let effect = root.join("effect");
        let security =
            super::security::start(&effect, matches!(scenario, Scenario::Denied)).await?;
        let rejections = Arc::new(Mutex::new(Vec::new()));
        let server = wire::server(
            kind,
            scenario,
            effect.clone(),
            session_id,
            rejections.clone(),
        )
        .await;
        let mut session = Session::from_provider_manager(manager);
        session.continue_enabled = true;
        session.continue_budget = 1;
        codelet_tools::set_continue_armed(session_id, true);
        if matches!(scenario, Scenario::Goal) {
            session.set_goal("complete the synthetic local goal", None);
            codelet_tools::set_session_goal(
                session_id,
                Some(codelet_tools::GoalSpec {
                    text: "complete the synthetic local goal".into(),
                    verify: None,
                }),
            );
        }
        Ok(Self {
            session,
            session_id,
            effect,
            rejections,
            kind,
            server,
            output: Output {
                cancel: matches!(scenario, Scenario::Cancel),
                ..Output::default()
            },
            _admission: admission,
            security,
            _runtime: fixture,
        })
    }

    pub async fn run(&mut self) -> TestResult {
        super::builders::run(self).await
    }

    pub(super) async fn drive<M: CompletionModel + 'static>(
        &mut self,
        agent: RigAgent<M>,
    ) -> TestResult
    where
        M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    {
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            run_agent_stream(
                agent,
                "Complete the synthetic local task",
                &mut self.session,
                self.output.interrupted.clone(),
                Arc::new(AtomicBool::new(false)),
                Arc::new(tokio::sync::Notify::new()),
                &self.output,
                self.session_id,
            ),
        )
        .await??;
        Ok(())
    }

    pub async fn security_checks(&self) -> TestResult<usize> {
        Ok(self
            .security
            .received_requests()
            .await
            .ok_or("security capture unavailable")?
            .iter()
            .filter(|request| request.url.path() == "/v1/classifier")
            .count())
    }

    pub async fn requests(&self) -> TestResult<Vec<String>> {
        let requests = self
            .server
            .received_requests()
            .await
            .ok_or("requests unavailable")?;
        for request in &requests {
            match self.kind {
                Kind::OAuth => assert_eq!(
                    request
                        .headers
                        .get("authorization")
                        .ok_or("OAuth header absent")?,
                    "Bearer sk-ant-oat-synthetic-only"
                ),
                Kind::Claude => assert_eq!(
                    request.headers.get("x-api-key").ok_or("API key absent")?,
                    "synthetic-loopback-only"
                ),
                Kind::Gemini => assert!(request
                    .url
                    .query_pairs()
                    .any(|(key, value)| key == "key" && value == "synthetic-loopback-only")),
                Kind::Zai => assert_eq!(
                    request
                        .headers
                        .get("authorization")
                        .ok_or("ZAI bearer absent")?,
                    "Bearer synthetic-loopback-only"
                ),
            }
        }
        requests
            .iter()
            .map(|request| Ok(String::from_utf8(request.body.clone())?))
            .collect()
    }

    pub fn assert_settled(&self, physical: u64, nudges: usize) -> TestResult {
        let events = self.output.events();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, StreamEvent::Done(_)))
                .count(),
            1
        );
        assert_eq!(events.iter().filter(|event| matches!(event, StreamEvent::ContinueState(state) if state.reason == ContinueStateReason::NudgeConsumed)).count(), nudges);
        assert_eq!(events.iter().filter(|event| matches!(event, StreamEvent::Status(text) if text.contains("native complete"))).count(), 1);
        let manifest = codelet_core::persistence::load_session(self.session_id)?;
        let billed = (
            manifest.token_usage.cumulative_billed_input,
            manifest.token_usage.cumulative_billed_output,
        );
        let expected = (12 * physical, 3 * physical);
        if billed != expected {
            return Err(format!(
                "{:?}: terminal billing {billed:?}, expected {expected:?}",
                self.kind
            )
            .into());
        }
        assert!(codelet_tools::take_done_acceptance(self.session_id).is_none());
        Ok(())
    }
}
