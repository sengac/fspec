/// The last message text of a session's inner (panics if empty / not User).
async fn last_message_text(session: &BackgroundSession) -> String {
    let inner = session.inner.lock().await;
    let last = inner
        .messages
        .last()
        .expect("a message must remain after the pin");
    match last {
        Message::User { content } => content
            .iter()
            .find_map(|c| match c {
                UserContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        _ => panic!("the pinned message must be a User message"),
    }
}

/// The task prompt the sub-agent's LLM request carried — recorded by
/// wiremock's request recorder (the last `/v1/chat/completions` body's
/// `messages` array, last message = the task). Process-global, tests
/// are serial.
async fn last_task_prompt(server: &MockServer) -> String {
    let requests = match server.received_requests().await {
        Some(requests) => requests,
        None => panic!("no requests recorded by the wiremock server"),
    };
    requests
        .iter()
        .rev()
        .find(|r| r.url.path() == "/v1/chat/completions")
        .map(|r| {
            let body: serde_json::Value =
                serde_json::from_slice(&r.body).expect("request body is JSON");
            let last = body["messages"]
                .as_array()
                .and_then(|m| m.last())
                .unwrap_or_else(|| panic!("request body has no messages: {body:?}"));
            // rig sends content as either a bare string or an array of
            // {type:"text", text:"..."} blocks — handle both.
            match last.get("content") {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Array(parts)) => parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join(""),
                other => panic!("unexpected content shape in last message: {other:?}"),
            }
        })
        .unwrap_or_else(|| {
            let paths: Vec<&str> = requests.iter().map(|r| r.url.path()).collect();
            panic!(
                "no /v1/chat/completions request recorded (paths: {paths:?}) — \
                 the sub-agent's LLM call must have reached the mock"
            )
        })
}

/// Mock LLM serving `dag_text` + a fresh fixture set.
async fn mock_llm_and_fixture_serving(
    dag_text: &str,
) -> (
    MockServer,
    tempfile::TempDir,
    String,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
    Arc<BackgroundSession>,
) {
    let server = MockServer::start().await;
    point_openai_at(&server.uri());
    let mock = Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(openai_body(dag_text)),
        )
        .expect(1..);
    server.register(mock).await;

    let (data_dir, project, manager, caller, target) = fixture_sessions().await;
    (server, data_dir, project, manager, caller, target)
}

/// Pull the last CompactionComplete event out of the target's buffer.
fn last_compaction_event(target: &BackgroundSession) -> StreamChunk {
    target
        .get_buffered_output(1000)
        .into_iter()
        .rev()
        .find(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
        .expect("a CompactionComplete event must have been emitted on the target")
}

// ============================================================================
// Scenario: GenerateCompaction tool wired into provider agent builders
// ============================================================================

/// Source-shape guard (the established provider-chain pattern): every
/// provider's create_rig_agent tool chain must include
/// GenerateCompactionTool::new(session_id) beside DeepSearchTool.
#[test]
fn generate_compaction_tool_wired_into_every_provider_tool_chain() {
    // @step Given all provider implementations exist (Claude, OpenAI, Gemini, Codex, ZAI, GitHub Copilot, custom provider)
    // @step When each provider's create_rig_agent() method builds an agent
    // @step Then GenerateCompactionTool::new(session_id) is included in the tool chain beside DeepSearchTool
    // @step And the parent agent can invoke GenerateCompaction like any other tool
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("repo rust root");

    let provider_files = [
        "rust/providers/src/claude.rs",
        "rust/providers/src/openai/agent.rs", // BUG-345: REFAC-031 moved the OpenAI agent wiring here
        "rust/providers/src/gemini.rs",
        "rust/providers/src/codex/agent.rs", // BUG-311: Codex agent wiring moved here
        "rust/providers/src/zai.rs",
        "rust/providers/src/copilot/rig_agent.rs",
        "rust/providers/src/custom/custom_provider.rs",
    ];

    for rel in provider_files {
        let src = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"));
        let deep_at = src
            .find("DeepSearchTool::new(session_id)")
            .unwrap_or_else(|| panic!("CMPCT-045: {rel} must wire DeepSearchTool"));
        let gc_at = src
            .find("GenerateCompactionTool::new(session_id)")
            .unwrap_or_else(|| {
                panic!("CMPCT-045: {rel} must wire GenerateCompactionTool beside DeepSearchTool")
            });
        assert!(
            gc_at > deep_at && (gc_at - deep_at) < 400,
            "CMPCT-045: in {rel} the GenerateCompactionTool wiring (char {gc_at}) must sit \
             beside the DeepSearchTool wiring (char {deep_at})"
        );
    }
}

/// Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
#[test]
fn generate_compaction_handler_registration_lifecycle_mirrors_deep_search() {
    // @step Given a background session is created in the agent loop
    // BUG-311: service handlers are registered in agent_loop/services.rs and
    // the per-turn CompactorRegistration releases them on drop.
    let read = |relative: &str| {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
    };
    let src = read("src/agent_loop/services.rs");
    let recovery = read("src/terminal_overflow_recovery.rs");

    let find = |needle: &str| -> usize {
        src.find(needle)
            .unwrap_or_else(|| panic!("CMPCT-045: '{needle}' not found in agent_loop/services.rs"))
    };

    // @step When the session's handlers are registered
    // @step Then a GenerateCompaction handler is registered for the session id capturing the provider, model, project path, and owning SessionManager
    let registration_at = find("register_generate_compaction_handler(");
    // Registered in the session-creation region, beside the DeepSearch
    // registration (within the same handler-registration block).
    let deep_search_at = find("register_deep_search_handler(");
    assert!(
        registration_at > deep_search_at && (registration_at - deep_search_at) < 3000,
        "CMPCT-045: the GenerateCompaction handler registration (char {registration_at})          must sit in the session-creation handler block, beside the DeepSearch          registration (char {deep_search_at})"
    );
    // The registration must capture the caller's BackgroundSession (for
    // status / progress / pending_dag_content) and the owning
    // SessionManager (for cross-session pinning).
    let capture_window = &src[registration_at..(registration_at + 400)];
    assert!(
        capture_window.contains("session.clone()"),
        "the GenerateCompaction registration must capture the caller's BackgroundSession"
    );
    assert!(
        capture_window.contains("owning_manager()"),
        "the GenerateCompaction registration must capture the owning SessionManager"
    );

    // @step And the handler is re-registered after model or provider changes
    // The registration block runs at the start of every turn and reads
    // provider/model/context-window/max-output from the session's
    // ProviderManager at call time (inside
    // `register_generate_compaction_handler`) — so after a /model or
    // /provider change the next turn re-registers with the fresh values,
    // exactly mirroring `register_deep_search_handler` (BUG-132).
    // Source-shape proof: the call passes `&inner_session` (the live
    // inner, not a cached provider string) into the registration.
    let call_window = &src[registration_at..(registration_at + 300)];
    assert!(
        call_window.contains("inner_session"),
        "the GenerateCompaction registration must read provider/model from the          session's live inner at call time (re-registration after /model /          /provider changes stays in sync — BUG-132 pattern)"
    );

    // @step And the handler is removed for the session id during end-of-turn cleanup
    find("set_deep_search_handler(session.id, None)");
    let drop_at = recovery
        .find("impl Drop for CompactorRegistration")
        .expect("CompactorRegistration releases its handlers on drop");
    assert!(
        recovery[drop_at..].contains("set_generate_compaction_handler(self.0, None)"),
        "dropping the turn's CompactorRegistration must remove the GenerateCompaction handler"
    );
}

// ============================================================================
// Scenario: Compactor sub-agent provider arms match the DeepSearch clone
// (CMPCT-046c source-shape: the compactor's built-in provider match must
// support the same arms as the DeepSearch sub-agent's match — including
// the github-copilot/copilot pending-builder arm — instead of falling
// through to "Unsupported provider")
// ============================================================================

#[test]
fn compactor_sub_agent_provider_arms_match_the_deep_search_clone() {
    // @step Given the compactor sub-agent's built-in provider match exists
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // BUG-311: the compactor's provider dispatch lives in compaction_builder.rs.
    let compactor = std::fs::read_to_string(manifest.join("src/compaction_builder.rs"))
        .unwrap_or_else(|e| panic!("failed to read compaction_builder.rs: {e}"));
    let deep_search = std::fs::read_to_string(manifest.join("src/deep_search_handler.rs"))
        .unwrap_or_else(|e| panic!("failed to read deep_search_handler.rs: {e}"));

    // @step When it is compared with the DeepSearch sub-agent's provider match
    let arms = [
        "\"claude\" =>",
        "\"openai\" =>",
        "\"gemini\" =>",
        "\"codex\" =>",
        "\"zai\" =>",
        "\"github-copilot\" | \"copilot\" =>",
    ];
    for arm in arms {
        assert!(
            deep_search.contains(arm),
            "the DeepSearch match must have arm {arm} (reference shape)"
        );
        assert!(
            compactor.contains(arm),
            "CMPCT-046c: the compactor match must have arm {arm}"
        );
    }

    // @step Then it supports the same arms: claude, openai, gemini, codex, zai, and github-copilot/copilot
    // (the loop above asserts each arm in both matches)

    // @step And the copilot arm returns the same distinct pending-builder error shape as DeepSearch instead of falling through to "Unsupported provider"
    assert!(
        compactor.contains("github-copilot compactor sub-agent builder pending"),
        "CMPCT-046c: the copilot arm must return its own distinct pending-builder error"
    );
    assert!(
        compactor.contains("Supported: claude, openai, gemini, codex, zai, github-copilot"),
        "CMPCT-046c: the 'Unsupported provider' fallthrough must name github-copilot as supported"
    );
}
