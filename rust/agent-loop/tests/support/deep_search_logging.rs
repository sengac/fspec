use codelet_test_helpers::setup_temp_directory;
use serde_json::json;
use std::{
    ffi::OsString,
    sync::{Arc, Mutex},
};
use tracing::{
    field::{Field, Visit},
    Event, Level, Subscriber,
};
use tracing_subscriber::{layer::Context, prelude::*, Layer};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub server: MockServer,
    previous_home: Option<OsString>,
}

impl Fixture {
    pub async fn new(success: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = setup_temp_directory()?;
        let server = MockServer::start().await;
        let providers = directory.path().join("providers");
        std::fs::create_dir_all(&providers)?;
        std::fs::write(
            providers.join("severity.rhai"),
            r#"
fn build_request(request) { #{ messages: request.messages, tools: request.tools } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_response(raw) { #{ content: "local answer", stop_reason: "end_turn" } }
fn parse_stream_chunk(config, data) { #{} }
fn define_tools(config) { [] }
fn format_system_prompt(config, preamble, fspec_guidance) { preamble }
"#,
        )?;
        std::fs::write(
            providers.join("severity.json"),
            json!({
                "name": "severity", "display_name": "Severity fixture", "base_url": server.uri(),
                "script": "severity.rhai", "auth": {"type": "bearer", "env_var": "BUG238_UNUSED"},
                "models": {"default": {"id": "severity-model", "context_window": 10000,
                    "max_output_tokens": 200, "supports_streaming": false, "supports_tools": true}},
                "tool_style": "openai", "api_style": "openai_chat"
            })
            .to_string(),
        )?;
        Mock::given(method("POST"))
            .respond_with(if success {
                ResponseTemplate::new(200).set_body_json(json!({}))
            } else {
                ResponseTemplate::new(400).set_body_string("fixture request rejected")
            })
            .mount(&server)
            .await;
        let previous_home = std::env::var_os("FSPEC_HOME");
        std::env::set_var("FSPEC_HOME", directory.path());
        Ok(Self {
            directory,
            server,
            previous_home,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        match &self.previous_home {
            Some(value) => std::env::set_var("FSPEC_HOME", value),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

#[derive(Clone, Default)]
pub struct Logs(pub Arc<Mutex<Vec<(Level, String)>>>);

#[derive(Default)]
struct Fields(String);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!(" {}={value:?}", field.name()));
    }
}

impl<SubscriberType: Subscriber> Layer<SubscriberType> for Logs {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, SubscriberType>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        if fields.0.contains("[deep-search") {
            self.records().push((*event.metadata().level(), fields.0));
        }
    }
}

impl Logs {
    fn records(&self) -> std::sync::MutexGuard<'_, Vec<(Level, String)>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn subscriber(&self) -> impl Subscriber + Send + Sync {
        tracing_subscriber::registry().with(self.clone())
    }

    pub fn assert_routine_debug(&self) {
        let logs = self.records();
        for message in [
            "execute_deep_search ENTER",
            "[deep-search-dispatch] ENTER",
            "routing provider=",
            "built agent for",
            "[deep-search-dispatch] EXIT",
        ] {
            let events: Vec<_> = logs
                .iter()
                .filter(|(_, text)| text.contains(message))
                .collect();
            assert!(
                !events.is_empty(),
                "missing routine diagnostic {message}: {logs:?}"
            );
            assert!(
                events.iter().all(|(level, _)| *level == Level::DEBUG),
                "routine diagnostics are not DEBUG: {events:?}"
            );
        }
    }

    pub fn assert_correlation(&self) -> TestResult {
        let logs = self.records();
        let entry = logs
            .iter()
            .find(|(_, text)| text.contains("execute_deep_search ENTER"))
            .ok_or("entry diagnostic missing")?;
        assert!(entry.1.contains("severity"));
        assert!(entry.1.contains("default"));
        assert!(entry.1.contains("ephemeral="));
        Ok(())
    }

    pub fn assert_no_incidents(&self) {
        assert!(self
            .records()
            .iter()
            .all(|(level, _)| *level != Level::WARN && *level != Level::ERROR));
    }

    pub fn assert_failure_warning(&self) {
        assert!(self.records().iter().any(|(level, text)| {
            (*level == Level::WARN || *level == Level::ERROR)
                && text.contains("custom-provider sub-agent prompt failed")
        }));
    }
}
