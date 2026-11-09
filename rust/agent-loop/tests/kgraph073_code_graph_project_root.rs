//! KGRAPH-073: GraphSearch indexes and queries the code graph of the
//! session's project root (or a root it names), never the daemon's
//! working directory.
//!
//! Feature: spec/features/the-code-graph-belongs-to-the-session-project-root.feature
//!
//! The test process plays the daemon: the environment sets its working
//! directory to a temp directory that holds a `daemon_only` function. Each
//! scenario builds a real `SessionManager` session over temp projects (an
//! isolated session is a session with a worktree path), registers the
//! production GraphSearch handler for it and calls GraphSearch the way an
//! agent does. DeepSearch runs against a loopback OpenAI-compatible
//! endpoint and its advertised tool list shows whether GraphSearch was
//! offered.

#[path = "support/bug268_environment.rs"]
mod environment;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_sessions::session_manager::SessionManager;
use environment::{Environment, TestResult};
use serde_json::{json, Value};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

type Failure = Box<dyn std::error::Error>;

/// The "daemon": the process working directory holds its own source.
fn daemon() -> TestResult<Environment> {
    let environment = Environment::new()?;
    write_functions(environment.directory.path(), &["daemon_only"])?;
    codelet_providers::stub_provider::register_stub_provider();
    Ok(environment)
}

fn write_functions(root: &Path, functions: &[&str]) -> TestResult {
    std::fs::create_dir_all(root.join("src"))?;
    let source: String = functions
        .iter()
        .map(|name| format!("def {name}():\n    return 1\n\n"))
        .collect();
    std::fs::write(root.join("src/module.py"), source)?;
    Ok(())
}

fn temp_project(functions: &[&str]) -> Result<(tempfile::TempDir, PathBuf), Failure> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let root = directory.path().join("project");
    write_functions(&root, functions)?;
    let root = root.canonicalize()?;
    Ok((directory, root))
}

/// A session over `project` (isolated when `worktree` is given) whose
/// GraphSearch handler is the production one.
async fn session(
    manager: &Arc<SessionManager>,
    project: &Path,
    worktree: Option<&Path>,
) -> Result<uuid::Uuid, Failure> {
    let id = uuid::Uuid::new_v4();
    manager
        .create_session_with_id(
            &id.to_string(),
            "stub/canned",
            project.to_str().ok_or("project path is not UTF-8")?,
            "kgraph073",
        )
        .await?;
    let session = manager.get_session(&id.to_string())?;
    *session.worktree_path.write().map_err(|e| e.to_string())? = worktree.map(Path::to_path_buf);
    codelet_agent_loop::graph_search_handler::register_for_session(&session);
    Ok(id)
}

fn manager() -> Result<Arc<SessionManager>, Failure> {
    let manager = SessionManager::new();
    manager.set_default_model("stub/canned")?;
    Ok(manager)
}

/// Call GraphSearch for a session the way an agent does.
fn graph_search(session: uuid::Uuid, args: Value) -> Result<Value, Failure> {
    let args: codelet_tools::GraphSearchArgs = serde_json::from_value(args)?;
    let output = codelet_tools::execute_graph_search(session, args.action);
    Ok(serde_json::from_str(&output)?)
}

fn names(result: &Value) -> Result<Vec<String>, Failure> {
    let mut names: Vec<String> = result["results"]
        .as_array()
        .ok_or_else(|| format!("no results in {result}"))?
        .iter()
        .filter_map(|row| row["name"].as_str().map(str::to_owned))
        .collect();
    names.sort();
    Ok(names)
}

fn search_only(session: uuid::Uuid) -> Result<Vec<String>, Failure> {
    let result = graph_search(
        session,
        json!({"action_type": "ast_search", "query": "only", "entity_type": "Function"}),
    )?;
    names(&result)
}

fn index(session: uuid::Uuid, extra: Value) -> Result<Value, Failure> {
    let mut args = json!({"action_type": "ast_index"});
    if let (Some(args), Some(extra)) = (args.as_object_mut(), extra.as_object()) {
        args.extend(extra.clone());
    }
    let result = graph_search(session, args)?;
    if result.get("error").is_some() {
        return Err(format!("ast_index failed: {result}").into());
    }
    Ok(result)
}

fn error_of(result: &Value) -> Result<String, Failure> {
    result["error"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("expected an error, got {result}").into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn sessions_in_two_projects_each_index_and_query_their_own_graph() -> TestResult {
    let _daemon = daemon()?;
    let manager = manager()?;

    // @step Given session A's project holds a function named alpha_only
    let (_a_directory, a_root) = temp_project(&["alpha_only"])?;
    let a = session(&manager, &a_root, None).await?;

    // @step And session B's project holds a function named beta_only
    let (_b_directory, b_root) = temp_project(&["beta_only"])?;
    let b = session(&manager, &b_root, None).await?;

    // @step When each session runs ast_index and then ast_search for "only"
    index(a, json!({}))?;
    index(b, json!({}))?;
    let a_names = search_only(a)?;
    let b_names = search_only(b)?;

    // @step Then session A's results list alpha_only and not beta_only
    assert_eq!(a_names, ["alpha_only"]);

    // @step And session B's results list beta_only and not alpha_only
    assert_eq!(b_names, ["beta_only"]);

    // @step And each project's graph is under that project's .fspec/graph directory
    assert!(a_root.join(".fspec/graph/ast-code.nano").is_dir());
    assert!(b_root.join(".fspec/graph/ast-code.nano").is_dir());
    assert!(!_daemon.directory.path().join(".fspec/graph").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn an_isolated_session_indexes_its_worktree_not_the_main_checkout() -> TestResult {
    let _daemon = daemon()?;
    let manager = manager()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let main = directory.path().join("main");
    let worktree = main.join(".fspec/worktrees/session");

    // @step Given an isolated session whose main checkout holds a function named main_only
    write_functions(&main, &["main_only"])?;

    // @step And whose worktree holds a function named tree_only
    write_functions(&worktree, &["tree_only"])?;
    let id = session(&manager, &main, Some(&worktree)).await?;

    // @step When the session runs ast_index and then ast_search for "only"
    index(id, json!({}))?;
    let found = search_only(id)?;

    // @step Then the results list tree_only and not main_only
    assert_eq!(found, ["tree_only"]);

    // @step And the graph is under the worktree's .fspec/graph directory
    assert!(worktree.join(".fspec/graph/ast-code.nano").is_dir());

    // @step And the main checkout has no .fspec/graph directory
    assert!(!main.join(".fspec/graph").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn indexing_another_root_builds_that_roots_own_graph() -> TestResult {
    let _daemon = daemon()?;
    let manager = manager()?;

    // @step Given a session whose project holds a function named project_fn and has been indexed
    let (_directory, root) = temp_project(&["project_fn"])?;
    let id = session(&manager, &root, None).await?;
    index(id, json!({}))?;

    // @step And the project's vendor/lib directory holds a function named vendor_fn
    write_functions(&root.join("vendor/lib"), &["vendor_fn"])?;

    // @step When the session runs ast_index with root "vendor/lib"
    let result = index(id, json!({"root": "vendor/lib"}))?;

    // @step Then the result names the canonical vendor/lib directory as its root
    let vendor = root.join("vendor/lib").canonicalize()?;
    assert_eq!(result["root"], vendor.display().to_string(), "{result}");

    // @step And ast_search in the project lists project_fn and not vendor_fn
    let project_search = graph_search(
        id,
        json!({"action_type": "ast_search", "query": "_fn", "entity_type": "Function"}),
    )?;
    assert_eq!(names(&project_search)?, ["project_fn"]);
    assert_eq!(project_search["root"], root.display().to_string());

    // @step And ast_search with root "vendor/lib" lists vendor_fn
    let vendor_search = graph_search(
        id,
        json!({"action_type": "ast_search", "query": "_fn", "entity_type": "Function",
               "root": "vendor/lib"}),
    )?;
    assert_eq!(names(&vendor_search)?, ["vendor_fn"]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn ast_index_refuses_the_path_parameter() -> TestResult {
    let _daemon = daemon()?;
    let manager = manager()?;

    // @step Given a session whose project holds source files
    let (_directory, root) = temp_project(&["some_fn"])?;
    let id = session(&manager, &root, None).await?;

    // @step When the session runs ast_index with path "src/**"
    let result = graph_search(id, json!({"action_type": "ast_index", "path": "src/**"}))?;

    // @step Then the tool fails with an error that names the root parameter
    let error = error_of(&result)?;
    assert!(error.contains("root"), "{error}");

    // @step And the project has no .fspec/graph directory
    assert!(!root.join(".fspec/graph").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_query_on_a_root_without_a_graph_fails_and_creates_nothing() -> TestResult {
    let _daemon = daemon()?;
    let manager = manager()?;

    // @step Given a session whose project has never been indexed
    let (_directory, root) = temp_project(&["never_fn"])?;
    let id = session(&manager, &root, None).await?;

    // @step When the session runs ast_search
    let result = graph_search(id, json!({"action_type": "ast_search", "query": "never"}))?;

    // @step Then the tool fails with "no code graph" naming the project root and ast_index
    let error = error_of(&result)?;
    assert!(error.contains("no code graph"), "{error}");
    assert!(error.contains(&root.display().to_string()), "{error}");
    assert!(error.contains("ast_index"), "{error}");

    // @step And the project has no .fspec/graph directory
    assert!(!root.join(".fspec/graph").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn an_isolated_session_cannot_use_the_main_checkout_as_a_graph_root() -> TestResult {
    let _daemon = daemon()?;
    let manager = manager()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let main = directory.path().join("main");
    let worktree = main.join(".fspec/worktrees/session");
    write_functions(&worktree, &["tree_only"])?;

    // @step Given an isolated session whose project is the main checkout
    write_functions(&main, &["main_only"])?;
    let id = session(&manager, &main, Some(&worktree)).await?;

    // @step When the session runs ast_search with the main checkout's absolute path as root
    let result = graph_search(
        id,
        json!({"action_type": "ast_search", "query": "only",
               "root": main.to_string_lossy()}),
    )?;

    // @step Then the tool fails with the 'blocked from original project' error
    let error = error_of(&result)?;
    assert!(error.contains("blocked from original project"), "{error}");
    assert!(!main.join(".fspec/graph").exists());
    Ok(())
}

// ── DeepSearch ──────────────────────────────────────────────────────

async fn loopback_llm() -> MockServer {
    let server = MockServer::start().await;
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    server
}

/// Runs a non-recursive DeepSearch for `project` and returns the tool
/// names the sub-agent was offered.
async fn deep_search_tools(server: &MockServer, project: &Path) -> Result<Vec<String>, Failure> {
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "kgraph073", "object": "chat.completion", "created": 1, "model": "o3",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "searched"},
                "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 2, "total_tokens": 12}
        })))
        .mount(server)
        .await;
    let answer = codelet_agent_loop::deep_search_handler::execute_deep_search(
        project,
        "where is the config loaded?",
        None,
        3,
        "openai",
        Some("o3"),
        0,
        0,
        Some(200_000),
        Some(100_000),
    )
    .await;
    let requests = server.received_requests().await.unwrap_or_default();
    let request = requests
        .iter()
        .find(|request| request.url.path().ends_with("/chat/completions"))
        .ok_or_else(|| format!("no sub-agent request was sent; answer: {answer:?}"))?;
    let body: Value = request.body_json()?;
    let mut names: Vec<String> = body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tool| tool["function"]["name"].as_str().or(tool["name"].as_str()))
        .map(str::to_owned)
        .collect();
    names.sort();
    Ok(names)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn deep_search_offers_graph_search_from_the_projects_graph_on_disk() -> TestResult {
    let environment = daemon()?;
    let server = loopback_llm().await;

    // @step Given project P's code graph is on disk and no graph is open in the process
    let (_p_directory, p_root) = temp_project(&["p_fn"])?;
    let indexed = codelet_graph::ast_index::dispatch_ast_index(&p_root, false, false).await;
    assert!(!indexed.contains("\"error\""), "{indexed}");
    codelet_graph::registry::close_all_graphs();

    // @step And project Q has no code graph and no learnings graph exists
    let (_q_directory, q_root) = temp_project(&["q_fn"])?;
    assert!(!q_root.join(".fspec/graph").exists());
    assert!(!environment.directory.path().join("graph").exists());

    // @step When a DeepSearch sub-agent is built for project P
    let p_tools = deep_search_tools(&server, &p_root).await?;

    // @step Then its tools include GraphSearch
    assert!(
        p_tools.iter().any(|name| name == "GraphSearch"),
        "{p_tools:?}"
    );

    // @step When P's graph is opened in the process and a DeepSearch sub-agent is built for project Q
    let _open = codelet_graph::registry::read_code_graph(&p_root).await?;
    let q_tools = deep_search_tools(&server, &q_root).await?;

    // @step Then its tools do not include GraphSearch
    assert!(
        !q_tools.iter().any(|name| name == "GraphSearch"),
        "{q_tools:?}"
    );
    Ok(())
}
