use super::environment::{Environment, TestResult};
use codelet_agent_loop::bridges::{register_agent_manager_handler, register_deep_search_handler};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_tools::{AgentManagerAction, AgentManagerResult, DeepSearchArgs, DeepSearchTool};
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::instrument::WithSubscriber;
use tracing::{field::Field, field::Visit, Event, Subscriber};
use tracing_subscriber::{layer::Context, prelude::*, Layer};
use uuid::Uuid;

pub const CLAUDE: &str = "anthropic/claude-sonnet-4-20250514";
const GOOGLE_KEY: &str = "GOOGLE_GENERATIVE_AI_API_KEY";

/// One parent session in an isolated, offline SessionManager.
pub struct Fixture {
    pub environment: Environment,
    pub manager: Arc<SessionManager>,
    pub id: Uuid,
}

impl Fixture {
    /// Given a session was created with `model` (registry seeded offline).
    pub async fn created_with(model: &str) -> TestResult<Self> {
        let environment = Environment::new()?;
        let root = environment.directory.path();
        let mut catalog: Value =
            serde_json::from_str(include_str!("../fixtures/prov101_models.json"))?;
        catalog["anthropic"]["models"]["claude-sonnet-4-20250514"] = json!({
            "id": "claude-sonnet-4-20250514", "name": "Claude Sonnet 4", "reasoning": true,
            "tool_call": true, "attachment": true, "temperature": true,
            "limit": {"context": 200000, "output": 64000}
        });
        std::fs::create_dir_all(root.join("cache"))?;
        std::fs::write(root.join("cache/models.json"), catalog.to_string())?;
        std::env::set_var(GOOGLE_KEY, "AIza-synthetic-loopback-only");
        let manager = SessionManager::new();
        manager.set_default_model(model)?;
        let id = Uuid::new_v4();
        let project = root.display().to_string();
        manager
            .create_session_with_id(&id.to_string(), model, &project, "inherit")
            .await?;
        Ok(Self {
            environment,
            manager,
            id,
        })
    }

    /// Registers the DeepSearch + AgentManager handlers from the live inner
    /// session — the exact calls the agent loop makes at the start of every
    /// turn (`agent_loop/services.rs::register_service_handlers`).
    pub async fn register_handlers(&self) -> TestResult {
        let session = self.manager.get_session(&self.id.to_string())?;
        let inner = session.inner.lock().await;
        let project = self.environment.directory.path().to_path_buf();
        register_deep_search_handler(self.id, &inner, project.clone());
        register_agent_manager_handler(
            self.id,
            &inner,
            project.display().to_string(),
            Some(self.manager.clone()),
        );
        Ok(())
    }

    /// The set_session_model RPC path (`SessionManagerHandle::set_model`).
    pub fn switch_model(&self, provider_id: &str, model_id: &str) -> TestResult {
        let handle: &dyn SessionManagerHandle = self.manager.as_ref();
        handle.set_model(&SessionId::new(self.id.to_string()), provider_id, model_id)?;
        Ok(())
    }

    /// Invokes the parent's DeepSearch tool and captures the sub-agent's
    /// entry diagnostic. `offline_key` is removed first so a cloud
    /// sub-agent fails at provider construction instead of dialling out.
    pub async fn deep_search(&self, offline_key: Option<&str>) -> (Result<String, String>, String) {
        if let Some(key) = offline_key {
            std::env::remove_var(key);
        }
        let logs = Logs::default();
        let args = DeepSearchArgs {
            query: "where is the config loaded?".into(),
            scope: None,
            max_depth: Some(2),
            max_recursion_depth: Some(0),
        };
        let result = DeepSearchTool::new(self.id)
            .call(args)
            .with_subscriber(tracing_subscriber::registry().with(logs.clone()))
            .await
            .map_err(|error| error.to_string());
        (result, logs.entry())
    }

    /// Spawns a subordinate through the parent's AgentManager handler and
    /// returns `(selected_model_string, context_window, max_output_tokens)`.
    pub async fn spawn(&self) -> TestResult<(Option<String>, Option<usize>, Option<usize>)> {
        let spawned = codelet_tools::agent_manager::handler::execute_agent_manager(
            self.id,
            AgentManagerAction::Spawn { role: None },
        );
        let session_id = match spawned {
            AgentManagerResult::Spawned { session_id } => session_id,
            other => return Err(format!("expected Spawned, got {other:?}").into()),
        };
        let subordinate = self.manager.get_session(&session_id)?;
        let inner = subordinate.inner.lock().await;
        let manager = inner.provider_manager();
        Ok((
            manager.selected_model_string(),
            manager.raw_model_context_window(),
            manager.raw_model_max_output_tokens(),
        ))
    }
}

/// Captures the `execute_deep_search ENTER` debug record.
#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<String>>>);

#[derive(Default)]
struct Fields(String);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!(" {}={value:?}", field.name()));
    }
}

impl<S: Subscriber> Layer<S> for Logs {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        if fields.0.contains("execute_deep_search ENTER") {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(fields.0);
        }
    }
}

impl Logs {
    fn entry(&self) -> String {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .join("\n")
    }
}
