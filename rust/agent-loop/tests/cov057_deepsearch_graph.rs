//! Feature: spec/features/deepsearch-graph-integration.feature
//!
//! COV-057: drives the real `execute_deep_search` against a loopback
//! OpenAI-compatible endpoint (`OPENAI_BASE_URL` → wiremock) and inspects
//! the sub-agent request it sends: the advertised tool list proves which
//! tools the sub-agent was built with, and the system message proves what
//! the DeepSearch system prompt contained. The Learnings graph is seeded on
//! disk in the temp data directory; DeepSearch reads its on-disk state.
//!
//! Recursion is disabled (`max_recursion_depth == depth`) so the toolset
//! is the 7 base read-only tools (+ GraphSearch when a graph is live); a
//! recursive build would add DeepSearch on top.

#[path = "support/bug268_environment.rs"]
mod environment;

use codelet_graph::database::GraphDatabase;
use codelet_graph::registry::reset_all_graphs;
use environment::{Environment, TestResult};
use serde_json::{json, Value};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

const LEARNINGS_SCHEMA: &str = include_str!("../../graph/schemas/learnings.pg");
const BASE_TOOLS: [&str; 7] = [
    "AstGrep",
    "Bash",
    "Glob",
    "Grep",
    "Ls",
    "Read",
    "SessionSearch",
];

struct Search {
    environment: Environment,
    server: MockServer,
}

impl Search {
    async fn new() -> TestResult<Self> {
        let environment = Environment::new()?;
        reset_all_graphs();
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "cov057", "object": "chat.completion", "created": 1, "model": "o3",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "searched"},
                    "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 2, "total_tokens": 12}
            })))
            .mount(&server)
            .await;
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        Ok(Self {
            environment,
            server,
        })
    }

    fn root(&self) -> &std::path::Path {
        self.environment.directory.path()
    }

    /// Runs a non-recursive DeepSearch and returns the sub-agent request body.
    async fn run(&self, query: &str) -> TestResult<(Result<String, String>, Value)> {
        let answer = codelet_agent_loop::deep_search_handler::execute_deep_search(
            self.root(),
            query,
            None,
            3,
            "openai",
            Some("o3"),
            0,
            0,
            // The model limits a parent session passes down.
            Some(200_000),
            Some(100_000),
        )
        .await;
        let requests = self.server.received_requests().await.unwrap_or_default();
        let request = requests
            .iter()
            .find(|request| request.url.path().ends_with("/chat/completions"))
            .ok_or_else(|| format!("no sub-agent request was sent; answer: {answer:?}"))?;
        Ok((answer, request.body_json()?))
    }

    async fn seed_learnings(&self, jsonl: &str) -> TestResult {
        let db_path = self.root().join("graph/learnings.nano");
        let db = GraphDatabase::init(&db_path, LEARNINGS_SCHEMA).await?;
        if !jsonl.is_empty() {
            db.load_jsonl(jsonl).await?;
        }
        // KGRAPH-073: availability is the on-disk graph; the seed handle closes.
        drop(db);
        Ok(())
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        reset_all_graphs();
    }
}

fn tool_names(body: &Value) -> Vec<String> {
    let mut names = body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tool| tool["function"]["name"].as_str().or(tool["name"].as_str()))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn system_prompt(body: &Value) -> String {
    body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|message| message["role"] == "system" || message["role"] == "developer")
        .map(|message| match &message["content"] {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Scenario: GraphSearch tool added when graph database is initialized
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_graph_search_tool_added_when_graph_database_is_initialized() -> TestResult {
    let search = Search::new().await?;

    // @step Given the knowledge graph database is initialized and available
    search.seed_learnings("").await?;

    // @step When a non-recursive DeepSearch sub-agent is being built
    let (answer, body) = search.run("where is the config loaded?").await?;
    assert_eq!(answer.as_deref(), Ok("searched"));

    // @step Then the GraphSearch tool is included in the sub-agent's toolset
    let names = tool_names(&body);
    assert!(names.iter().any(|name| name == "GraphSearch"), "{names:?}");

    // @step And the sub-agent has 8 tools total
    assert_eq!(names.len(), 8, "{names:?}");
    Ok(())
}

/// Scenario: DeepSearch works without graph database
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_deep_search_works_without_graph_database() -> TestResult {
    let search = Search::new().await?;

    // @step Given no knowledge graph database exists
    assert!(!search.root().join("graph").exists());
    assert!(!codelet_graph::registry::learnings_graph_available());

    // @step When a non-recursive DeepSearch sub-agent is being built
    let (answer, body) = search.run("where is the config loaded?").await?;

    // @step Then the sub-agent has the default 7 tools
    assert_eq!(tool_names(&body), BASE_TOOLS);

    // @step And no error is raised
    assert_eq!(answer.as_deref(), Ok("searched"));
    Ok(())
}

/// Scenario: Graph context injected into system prompt when data exists
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_graph_context_injected_into_system_prompt_when_data_exists() -> TestResult {
    let search = Search::new().await?;

    // @step Given the learnings graph contains learnings related to the search query
    let learning = json!({"type": "Learning", "data": {
        "slug": "bcrypt-hashing", "title": "Use bcrypt for password hashing",
        "category": "convention", "confidence": "high",
        "content": "All authentication services must use bcrypt with cost factor 12",
        "firstSeen": "2026-01-01T00:00:00Z", "lastSeen": "2026-01-01T00:00:00Z",
        "mentionCount": 1
    }});
    search.seed_learnings(&learning.to_string()).await?;

    // @step When the DeepSearch system prompt is constructed
    let (answer, body) = search.run("authentication").await?;
    assert_eq!(answer.as_deref(), Ok("searched"));
    let prompt = system_prompt(&body);

    // @step Then a learnings context section is appended to the prompt
    let section = prompt
        .find("# Learnings Context")
        .ok_or_else(|| format!("no learnings section in system prompt: {prompt}"))?;
    assert!(
        prompt[..section].contains("SessionSearch"),
        "context must follow the base DeepSearch prompt"
    );

    // @step And the context includes the related learning titles and categories
    let context = &prompt[section..];
    assert!(
        context.contains("Use bcrypt for password hashing"),
        "{context}"
    );
    assert!(context.contains("[convention]"), "{context}");
    Ok(())
}
