//! Feature: spec/features/claude-effort-can-change-between-turns.feature
//!
//! MODEL-015: a Claude level changed between turns is an effort-only message
//! before the user's next message on a model that takes per-message effort,
//! and a top-level change otherwise. Each turn goes the production way: the
//! effort log plans the turn (`EffortLog::plan_turn`, or the agent loop's
//! `plan_turn_effort` where persistence matters), the Claude provider builds
//! the turn's agent with the level's thinking parameters, and the
//! stream-loop driver's `configure_turn_effort` gives it the turn's params.
//! The requests are captured by a local Anthropic endpoint.

#[path = "fixtures/offline_catalog.rs"]
mod offline_catalog;

use codelet_cli::session::effort::configure_turn_effort;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::RigAgent;
use codelet_providers::claude::{build_beta_headers, AuthMode, ClaudeProvider};
use codelet_providers::claude_effort::{EffortLog, PER_MESSAGE_EFFORT_BETA};
use codelet_providers::models::{ReasoningCapability, ReasoningLevel};
use codelet_providers::thinking_wire::{ThinkingTarget, ThinkingWire, TurnLevel};
use futures::StreamExt;
use rig::message::Message;
use rig::providers::anthropic;
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type ClaudeModel = anthropic::completion::CompletionModel<
    codelet_providers::claude_refreshing_client::RefreshingClaudeClient,
>;

const REPLY: &str = concat!(
    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_model015\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n",
    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"done\"}}\n\n",
    "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
    "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":1}}\n\n",
    "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
);

/// A captured request: its body and its `anthropic-beta` header.
struct Request {
    body: Value,
    betas: String,
}

impl Request {
    fn top_level_effort(&self) -> Option<&str> {
        self.body["output_config"]["effort"].as_str()
    }

    /// The request's messages, without the prompt-cache breakpoint (which
    /// moves to the last message on every request).
    fn messages(&self) -> Vec<Value> {
        let mut messages = self.body["messages"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for message in &mut messages {
            if let Some(Value::Array(blocks)) = message.get_mut("content") {
                for block in blocks.iter_mut().filter_map(Value::as_object_mut) {
                    block.remove("cache_control");
                }
            }
        }
        messages
    }

    /// The efforts of the request's effort-only messages, in order.
    fn effort_messages(&self) -> Vec<String> {
        self.messages()
            .iter()
            .filter(|message| message["role"] == "system")
            .filter_map(|message| message["output_config"]["effort"].as_str())
            .map(str::to_string)
            .collect()
    }

    /// The message right before the user message whose text is `text`.
    fn before_user_message(&self, text: &str) -> Option<Value> {
        let messages = self.messages();
        let at = messages.iter().position(|message| {
            message["role"] == "user" && message.to_string().contains(&format!("\"{text}\""))
        })?;
        messages.get(at.checked_sub(1)?).cloned()
    }
}

fn effort_message(effort: &str) -> Value {
    json!({"role": "system", "content": [], "output_config": {"effort": effort}})
}

async fn endpoint() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(REPLY),
        )
        .mount(&server)
        .await;
    server
}

async fn last_request(server: &MockServer) -> TestResult<Request> {
    let requests = server.received_requests().await.ok_or("no requests")?;
    let request = requests.last().ok_or("no request was sent")?;
    Ok(Request {
        body: serde_json::from_slice(&request.body)?,
        betas: request
            .headers
            .get("anthropic-beta")
            .map(|value| value.to_str().map(str::to_string))
            .transpose()?
            .unwrap_or_default(),
    })
}

/// A Claude model on the Anthropic wire, advertising Low to Max.
fn target(api_model: &str) -> ThinkingTarget {
    let efforts = ["low", "medium", "high", "xhigh", "max"].map(String::from);
    ThinkingTarget::new(
        Ok(ThinkingWire::Anthropic),
        &format!("anthropic/{api_model}"),
        api_model,
        Ok(Some(ReasoningCapability::declared(
            &efforts, None, None, None,
        ))),
    )
}

/// The thinking parameters of a turn at `level` (`None`: Off).
fn config(target: &ThinkingTarget, level: Option<&str>) -> TestResult<Option<Value>> {
    let level = match level {
        Some(name) => TurnLevel::Level(ReasoningLevel::effort(name)?),
        None => TurnLevel::Off,
    };
    Ok(target.config_for(Some(&level))?)
}

/// A conversation on one Claude model, held the way the session holds it.
struct Conversation {
    server: MockServer,
    api_model: String,
    target: ThinkingTarget,
    auth: AuthMode,
    log: EffortLog,
    history: Vec<Message>,
}

impl Conversation {
    async fn on(api_model: &str, auth: AuthMode) -> Self {
        Self {
            server: endpoint().await,
            api_model: api_model.to_string(),
            target: target(api_model),
            auth,
            log: EffortLog::default(),
            history: Vec::new(),
        }
    }

    fn provider(&self) -> TestResult<ClaudeProvider> {
        let key = match self.auth {
            AuthMode::OAuth => "sk-ant-oat01-model015",
            _ => "sk-ant-api03-model015",
        };
        let source =
            ClaudeProvider::from_api_key_with_mode_and_model(key, self.auth, &self.api_model)?;
        let client = anthropic::Client::<
            codelet_providers::claude_refreshing_client::RefreshingClaudeClient,
        >::builder()
        .api_key(key)
        .base_url(self.server.uri())
        .http_headers(source.client().headers().clone())
        .http_client(source.client().http_client().clone())
        .build()?;
        Ok(ClaudeProvider::from_test_client(
            client,
            &self.api_model,
            self.auth,
        )?)
    }

    /// The turn's agent, built at `level` and configured by the driver.
    fn agent(&mut self, level: Option<&str>) -> TestResult<RigAgent<ClaudeModel>> {
        let config = config(&self.target, level)?;
        let built = self
            .provider()?
            .create_rig_agent(Uuid::new_v4(), None, config);
        Ok(configure_turn_effort(
            &mut self.log,
            RigAgent::with_default_depth(built),
            &self.history,
        ))
    }

    /// The user sends `text` at `level` (a fresh user input).
    async fn send(&mut self, level: Option<&str>, text: &str) -> TestResult<Request> {
        let config = config(&self.target, level)?;
        self.log.plan_turn(
            self.target.takes_per_message_effort(),
            config.as_ref(),
            Uuid::new_v4(),
            true,
        );
        let agent = self.agent(level)?;
        request(&agent, text, &mut self.history).await?;
        self.history.push(Message::user(text));
        self.history.push(Message::assistant("done"));
        last_request(&self.server).await
    }
}

/// Sends one request of a turn through `agent` and drains its stream.
async fn request(agent: &RigAgent<ClaudeModel>, text: &str, history: &mut [Message]) -> TestResult {
    let stream = agent.prompt_streaming_with_history(text, history).await;
    futures::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        item?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn an_effort_change_on_a_model_that_takes_per_message_effort_keeps_the_cache() -> TestResult {
    // @step Given a conversation on anthropic/claude-opus-5-5 that started at effort high
    let mut conversation = Conversation::on("claude-opus-5-5", AuthMode::ApiKey).await;
    let first = conversation.send(Some("high"), "first question").await?;
    assert_eq!(first.top_level_effort(), Some("high"));
    assert!(first.effort_messages().is_empty());

    // @step When the user chooses low and sends the next message
    let second = conversation.send(Some("low"), "second question").await?;

    // @step Then the request keeps top-level effort high and carries the beta header mid-conversation-output-config-2026-07-01
    assert_eq!(second.top_level_effort(), Some("high"));
    assert_eq!(
        second.body["thinking"],
        json!({"type": "adaptive", "display": "summarized"})
    );
    assert!(
        second
            .betas
            .split(',')
            .any(|beta| beta == PER_MESSAGE_EFFORT_BETA),
        "{}",
        second.betas
    );
    assert!(!first.betas.contains(PER_MESSAGE_EFFORT_BETA));

    // @step And an effort-only system message with effort low sits right before the new user message
    assert_eq!(
        second.before_user_message("second question"),
        Some(effort_message("low"))
    );
    assert_eq!(second.effort_messages(), vec!["low"]);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn earlier_effort_messages_stay_in_place() -> TestResult {
    // @step Given that conversation already changed to low before an earlier user message
    let mut conversation = Conversation::on("claude-opus-5-5", AuthMode::ApiKey).await;
    conversation.send(Some("high"), "first question").await?;
    let earlier = conversation.send(Some("low"), "second question").await?;

    // @step When the user chooses xhigh and sends the next message
    let next = conversation.send(Some("xhigh"), "third question").await?;

    // @step Then the request still holds the low effort message at its position
    let earlier_messages = earlier.messages();
    let next_messages = next.messages();
    assert_eq!(
        next_messages.get(..earlier_messages.len()),
        Some(earlier_messages.as_slice()),
        "the earlier request's messages are an unchanged prefix"
    );
    assert_eq!(
        next.before_user_message("second question"),
        Some(effort_message("low"))
    );
    assert_eq!(next.top_level_effort(), Some("high"));

    // @step And a new xhigh effort message sits right before the new user message
    assert_eq!(
        next.before_user_message("third question"),
        Some(effort_message("xhigh"))
    );
    assert_eq!(next.effort_messages(), vec!["low", "xhigh"]);
    Ok(())
}

fn user_envelope(id: Uuid, text: &str) -> TestResult<String> {
    Ok(serde_json::to_string(&json!({
        "uuid": id,
        "message": {"role": "user", "content": [{"type": "text", "text": text}]}
    }))?)
}

fn assistant_envelope(text: &str) -> TestResult<String> {
    Ok(serde_json::to_string(&json!({
        "uuid": Uuid::new_v4(),
        "message": {"role": "assistant", "content": [{"type": "text", "text": text}]}
    }))?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn effort_messages_survive_a_resume() -> TestResult {
    // @step Given a conversation on anthropic/claude-opus-5-5 that changed to low before an earlier user message
    let data_dir = codelet_test_helpers::shared_temp_subdirectory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir)?;
    codelet_core::persistence::reset_stores_for_tests();
    let manager = codelet_sessions::SessionManager::new();
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    offline_catalog::seed_offline_catalog()?;
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session_checked(None)?;
    let session_id = Uuid::parse_str(&sid.value)?;
    let session = manager.get_session(&sid.value)?;
    let mut conversation = Conversation::on("claude-opus-5-5", AuthMode::ApiKey).await;
    let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
    {
        let mut inner = session.inner.lock().await;
        for (id, level, text) in [
            (first, "high", "first question"),
            (second, "low", "second question"),
        ] {
            let config = config(&conversation.target, Some(level))?;
            codelet_agent_loop::claude_effort::plan_turn_effort(
                session_id,
                &mut inner,
                &conversation.target,
                config.as_ref(),
                id,
                true,
            )
            .await?;
            conversation.log = inner.effort_log.clone();
            let agent = conversation.agent(Some(level))?;
            request(&agent, text, &mut conversation.history).await?;
            inner.effort_log = conversation.log.clone();
            conversation.history.push(Message::user(text));
            conversation.history.push(Message::assistant("done"));
        }
        // A restarted process holds no effort log until the session is restored.
        inner.effort_log = EffortLog::default();
    }

    // @step When the session is restored and the user sends the next message
    handle.restore_session_messages(
        &sid,
        vec![
            user_envelope(first, "first question")?,
            assistant_envelope("done")?,
            user_envelope(second, "second question")?,
            assistant_envelope("done")?,
        ],
    )?;
    let next = {
        let mut inner = session.inner.lock().await;
        let config = config(&conversation.target, Some("low"))?;
        codelet_agent_loop::claude_effort::plan_turn_effort(
            session_id,
            &mut inner,
            &conversation.target,
            config.as_ref(),
            Uuid::new_v4(),
            true,
        )
        .await?;
        conversation.log = inner.effort_log.clone();
        conversation.history = inner.messages.clone();
        let agent = conversation.agent(Some("low"))?;
        request(&agent, "third question", &mut conversation.history).await?;
        last_request(&conversation.server).await?
    };

    // @step Then the low effort message is sent at its position unchanged
    assert_eq!(
        next.before_user_message("second question"),
        Some(effort_message("low"))
    );
    assert_eq!(next.effort_messages(), vec!["low"]);
    assert_eq!(next.top_level_effort(), Some("high"));
    assert!(next.betas.contains(PER_MESSAGE_EFFORT_BETA));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_model_without_per_message_effort_changes_the_top_level_effort() -> TestResult {
    // @step Given a conversation on anthropic/claude-opus-4-8 that started at effort high
    let mut conversation = Conversation::on("claude-opus-4-8", AuthMode::ApiKey).await;
    let first = conversation.send(Some("high"), "first question").await?;
    assert_eq!(first.top_level_effort(), Some("high"));

    // @step When the user chooses low and sends the next message
    let second = conversation.send(Some("low"), "second question").await?;

    // @step Then the request's top-level effort is low
    assert_eq!(second.top_level_effort(), Some("low"));

    // @step And it carries no effort message and no per-message effort beta header
    assert!(second.effort_messages().is_empty());
    assert!(
        !second.betas.contains(PER_MESSAGE_EFFORT_BETA),
        "{}",
        second.betas
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_thinking_change_goes_at_the_top_level() -> TestResult {
    // @step Given a conversation on anthropic/claude-sonnet-5-5 that started with thinking off
    let mut conversation = Conversation::on("claude-sonnet-5-5", AuthMode::ApiKey).await;
    let first = conversation.send(None, "first question").await?;
    assert_eq!(first.body["thinking"], json!({"type": "between_tools"}));

    // @step When the user chooses medium and sends the next message
    let second = conversation.send(Some("medium"), "second question").await?;

    // @step Then the request's top-level thinking is adaptive with effort medium
    assert_eq!(
        second.body["thinking"],
        json!({"type": "adaptive", "display": "summarized"})
    );
    assert_eq!(second.top_level_effort(), Some("medium"));

    // @step And it carries no effort message
    assert!(second.effort_messages().is_empty());
    assert!(!second.betas.contains(PER_MESSAGE_EFFORT_BETA));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_change_during_a_running_turn_applies_from_the_next_user_message() -> TestResult {
    // @step Given a turn on anthropic/claude-opus-5-5 at effort high is running a tool loop
    let mut conversation = Conversation::on("claude-opus-5-5", AuthMode::ApiKey).await;
    conversation.send(Some("high"), "first question").await?;
    let config = config(&conversation.target, Some("high"))?;
    conversation.log.plan_turn(
        conversation.target.takes_per_message_effort(),
        config.as_ref(),
        Uuid::new_v4(),
        true,
    );
    let running = conversation.agent(Some("high"))?;
    request(&running, "run the tools", &mut conversation.history).await?;
    conversation.history.push(Message::user("run the tools"));
    conversation
        .history
        .push(Message::assistant("calling a tool"));

    // @step When the user chooses low before the loop's next request
    let chosen = "low";

    // @step Then the loop's next request carries no new effort message
    request(&running, "tool results", &mut conversation.history).await?;
    let loop_request = last_request(&conversation.server).await?;
    assert!(loop_request.effort_messages().is_empty());
    assert_eq!(loop_request.top_level_effort(), Some("high"));
    conversation.history.push(Message::user("tool results"));
    conversation.history.push(Message::assistant("done"));

    // @step And the user's next message is preceded by the low effort message
    let next = conversation.send(Some(chosen), "next question").await?;
    assert_eq!(
        next.before_user_message("next question"),
        Some(effort_message("low"))
    );
    assert_eq!(next.effort_messages(), vec!["low"]);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn after_compaction_the_level_in_effect_is_declared_at_the_top_level() -> TestResult {
    // @step Given a conversation on anthropic/claude-opus-5-5 that started at high and changed to low
    let mut conversation = Conversation::on("claude-opus-5-5", AuthMode::ApiKey).await;
    conversation.send(Some("high"), "first question").await?;
    let changed = conversation.send(Some("low"), "second question").await?;
    assert_eq!(changed.effort_messages(), vec!["low"]);

    // @step When the conversation is compacted and the user sends the next message
    let config = config(&conversation.target, Some("low"))?;
    conversation.log.plan_turn(
        conversation.target.takes_per_message_effort(),
        config.as_ref(),
        Uuid::new_v4(),
        true,
    );
    let turn = conversation.agent(Some("low"))?;
    conversation.history = vec![
        Message::user("<summary>first and second question answered</summary>"),
        Message::assistant("understood"),
    ];
    // A compaction inside the running turn: the turn's later requests.
    request(&turn, "Continue", &mut conversation.history).await?;
    let within_turn = last_request(&conversation.server).await?;
    // The next turn after the compaction.
    let next = conversation.send(Some("low"), "third question").await?;

    // @step Then the request's top-level effort is low
    assert_eq!(within_turn.top_level_effort(), Some("low"));
    assert_eq!(next.top_level_effort(), Some("low"));

    // @step And no effort message from before the compaction is sent
    assert!(within_turn.effort_messages().is_empty());
    assert!(next.effort_messages().is_empty());
    assert!(!next.betas.contains(PER_MESSAGE_EFFORT_BETA));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn subscription_requests_carry_the_per_message_effort_beta_header_too() -> TestResult {
    // @step Given a conversation on anthropic/claude-opus-5-5 signed in with the Claude subscription (OAuth)
    let saved = std::env::var_os("FSPEC_CLAUDE_CODE_VERSION");
    std::env::set_var("FSPEC_CLAUDE_CODE_VERSION", "2.1.316");
    let mut conversation = Conversation::on("claude-opus-5-5", AuthMode::OAuth).await;
    let result = async {
        conversation.send(Some("high"), "first question").await?;

        // @step When the user changes effort and sends the next message
        conversation.send(Some("low"), "second question").await
    }
    .await;
    match saved {
        Some(value) => std::env::set_var("FSPEC_CLAUDE_CODE_VERSION", value),
        None => std::env::remove_var("FSPEC_CLAUDE_CODE_VERSION"),
    }
    let second = result?;

    // @step Then the request carries the beta header mid-conversation-output-config-2026-07-01 with the subscription beta headers
    let betas: Vec<&str> = second.betas.split(',').collect();
    for beta in build_beta_headers("claude-opus-5-5", true).split(',') {
        assert!(betas.contains(&beta), "{beta} missing from {betas:?}");
    }
    assert!(betas.contains(&"oauth-2025-04-20"));
    assert!(betas.contains(&"claude-code-20250219"));
    assert!(betas.contains(&PER_MESSAGE_EFFORT_BETA), "{betas:?}");
    assert_eq!(
        second.before_user_message("second question"),
        Some(effort_message("low"))
    );
    Ok(())
}
