// ============================================================================
// Process-global guard — env vars (OPENAI_*), the data directory, and the
// tool-handler registries are process-global, so every test fences on this.
// ============================================================================

static GLOBAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ============================================================================
// Shared fixtures
// ============================================================================

const OPENAI_MODEL: &str = "o3";

/// A minimal but parseable compaction DAG (one D1 node + dag-files).
const SUCCESS_DAG: &str = r#"<dag-node depth="D1" turns="0-5" label="Work arc">
- completed the task
</dag-node>
<dag-files>
- src/foo.rs (Modified)
</dag-files>"#;

/// OpenAI-style non-streaming completion response body (all required
/// fields of the patched rig `CompletionResponse`: id, object, created,
/// model, choices; `system_fingerprint`/`usage` are optional).
fn openai_body(text: &str) -> String {
    format!(
        r#"{{"id":"chatcmpl-test","object":"chat.completion","created":1,"model":"o3","choices":[{{"index":0,"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}}}}"#,
        serde_json::to_string(text).expect("DAG text is JSON-serializable")
    )
}

/// Point OPENAI_* at the wiremock server (serial — process-global env).
fn point_openai_at(base_url: &str) {
    std::env::set_var("OPENAI_API_KEY", "sk-test-dummy");
    std::env::set_var("OPENAI_BASE_URL", base_url);
    std::env::set_var("OPENAI_MODEL", OPENAI_MODEL);
}

/// Create a hermetic SessionManager + two live sessions (caller +
/// target) under a fresh data directory. The model cache is pre-seeded
/// from the offline models.dev fixture (prov101) so registry-backed
/// model resolution stays fully offline (the `cmpct041` / `rpc386`
/// pattern). Returns the data dir (the caller must keep it alive).
async fn fixture_sessions() -> (
    tempfile::TempDir,
    String,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
    Arc<BackgroundSession>,
) {
    // Set dummy creds so `ProviderCredentials::detect()` passes offline.
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");

    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(
        cache_dir.join("models.json"),
        include_str!("../fixtures/prov101_models.json"),
    )
    .expect("write models fixture");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data dir");
    codelet_core::persistence::reset_stores_for_tests();

    let manager = SessionManager::new();
    assert_eq!(manager.set_default_model("openai/o3"), Ok(()));

    let project = tempfile::tempdir()
        .expect("project tempdir")
        .keep()
        .to_str()
        .expect("utf8")
        .to_string();
    // NOTE: the project tempdir is intentionally leaked per test — the
    // path must outlive the sessions. Each test leaks one tempdir;
    // acceptable for a test binary.

    let caller_id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &caller_id.to_string(),
            "openai/o3",
            &project,
            "cmpct045-caller",
        )
        .await
        .expect("create caller session");
    let target_id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &target_id.to_string(),
            "openai/o3",
            &project,
            "cmpct045-target",
        )
        .await
        .expect("create target session");

    let caller = manager
        .get_session(&caller_id.to_string())
        .expect("caller live");
    let target = manager
        .get_session(&target_id.to_string())
        .expect("target live");
    (data_dir, project, manager, caller, target)
}

/// Seed `n` compactable conversation turns into a session's inner and
/// set the token tracker to a large pre-compaction basis (190k).
/// BUG-311: self-target compaction returns a queued candidate (CMPCT-052).
fn queued_content(result: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(result).expect("queued result JSON");
    assert_eq!(value["status"], "queued", "{result}");
    value["content"]
        .as_str()
        .expect("queued DAG content")
        .to_owned()
}

/// BUG-311: candidates are validated against persisted originals, so persist
/// the six messages (turns 0-5) that SUCCESS_DAG covers.
fn persist_covered_turns(session: &BackgroundSession) {
    let mut manifest =
        codelet_core::persistence::load_session(session.id).expect("persisted session");
    let mut turn = 0;
    while manifest.messages.len() < 6 {
        let role = if manifest.messages.len().is_multiple_of(2) {
            "user"
        } else {
            "assistant"
        };
        codelet_core::persistence::append_message(
            &mut manifest,
            role,
            &format!("{role} turn {turn}"),
        )
        .expect("persist covered original");
        turn += 1;
    }
}

/// BUG-311: durable commits admit through the session's selected preparer.
async fn install_selected(inner: &mut codelet_cli::session::Session, id: uuid::Uuid) {
    inner
        .provider_manager_mut()
        .set_model_direct("openai", "o3", Some(200_000), Some(4096), None)
        .expect("explicit selected target limits");
    selected::install(inner, id).await;
}

async fn seed_turns(session: &BackgroundSession, n: usize) {
    let mut inner = session.inner.lock().await;
    install_selected(&mut inner, session.id).await;
    for i in 0..n {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("user turn {i} do work"))),
        });
        inner.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
                text: format!("assistant turn {i} did work"),
            })),
        });
        inner.turns.push(ConversationTurn {
            user_message: format!("user turn {i}"),
            tool_calls: vec![],
            tool_results: vec![],
            assistant_response: format!("assistant turn {i}"),
            tokens: 500,
            timestamp: std::time::SystemTime::now(),
            previous_error: None,
        });
    }
    inner.token_tracker.input_tokens = 190_000;
    drop(inner);
    persist_covered_turns(session);
}

/// BUG-311: seed a committed durable DAG on `session` through the real commit,
/// publish and restore path, with `covered` persisted originals.
async fn commit_existing_dag(session: &BackgroundSession, dag: &str, covered: usize) {
    use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
    use codelet_core::persistence::compaction_boundary::{
        capture_compaction_source, CompactionCheckpoint, CompactionTokenProvenance,
    };
    use codelet_core::persistence::compaction_commit::{
        commit_compaction, publish_compaction, CompactionCandidate,
    };
    let mut inner = session.inner.lock().await;
    install_selected(&mut inner, session.id).await;
    let model = inner.current_model_id().expect("selected model");
    let mut manifest =
        codelet_core::persistence::load_session(session.id).expect("persisted session");
    while manifest.messages.len() < covered {
        let role = if manifest.messages.len().is_multiple_of(2) {
            "user"
        } else {
            "assistant"
        };
        codelet_core::persistence::append_message(&mut manifest, role, &format!("{role} original"))
            .expect("persist original");
    }
    let source = capture_compaction_source(&manifest, &model, "dag-v1").expect("source");
    let dag = validate_dag_candidate(
        &format!("{dag}\n<dag-files>\n</dag-files>"),
        &DagValidationContext {
            persisted_turn_count: covered,
            required_coverage_end: covered.checked_sub(1),
            ..Default::default()
        },
    )
    .expect("valid committed DAG");
    let candidate = CompactionCandidate {
        operation_id: uuid::Uuid::new_v4(),
        checkpoint: CompactionCheckpoint {
            provider: manifest.provider.clone(),
            model: model.clone(),
            generation: source.generation + 1,
            payload: serde_json::json!({"lineage":"explicit-full-DAG"}),
        },
        source,
        dag,
        input_tokens: 23,
        output_reservation: 8,
        token_provenance: CompactionTokenProvenance::Estimated,
    };
    let promoted = commit_compaction(candidate, &model, |_, _| Ok(())).expect("commit");
    publish_compaction(&promoted, |_| Ok(())).expect("publish");
    let manifest = codelet_core::persistence::load_session(session.id).expect("committed");
    inner
        .restore_compaction_boundary(&manifest)
        .expect("restore committed DAG");
}

/// Seed an existing wrapped compaction DAG (ending at turn 45) into a
/// session's inner so `detect_existing_dag` finds it.
async fn seed_existing_dag(session: &BackgroundSession, dag: &str, turns_before: usize) {
    let mut inner = session.inner.lock().await;
    for i in 0..turns_before {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("early turn {i}"))),
        });
    }
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(wrap_dag_content(dag))),
    });
    inner.token_tracker.input_tokens = 150_000;
}

/// Register the real GenerateCompaction handler for `caller` and return
/// the tool the agent would invoke.
fn register_and_build_tool(
    caller: &Arc<BackgroundSession>,
    project: &str,
    manager: &Arc<SessionManager>,
) -> GenerateCompactionTool {
    {
        // The handler needs a sync &Session — take the lock briefly.
        let guard = caller.inner.try_lock().expect("caller inner free");
        register_generate_compaction_handler(
            caller.id,
            &guard,
            std::path::PathBuf::from(project),
            caller.clone(),
            Some(manager.clone()),
        );
        // drop(guard) here — the handler closure captures Arcs, not the lock.
    }
    GenerateCompactionTool::new(caller.id)
}
