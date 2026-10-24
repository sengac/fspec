#![doc = "Feature: spec/features/codex-interruption-outcome-recording.feature"]

#[path = "../../cli/tests/fixtures/committed_interruption.rs"]
mod committed_interruption;
#[path = "../../cli/tests/fixtures/interruption_tool.rs"]
mod interruption_tool;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

use codelet_agent_loop::{persist::persist_user_message, BackgroundOutput};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::lifecycle_hooks::{
    CompiledHookCommand, CompiledHookGroup, CompiledLifecycleHooks, HookMatcher,
};
use codelet_core::persistence::{
    create_session_with_provider, get_session_message_envelopes, reset_stores_for_tests,
};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_sessions::background_session::BackgroundSession;
use codelet_test_helpers::setup_temp_directory;
use serde_json::Value;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use transport::TestResult;

struct RecordingOutput {
    background: BackgroundOutput,
    events: Mutex<Vec<StreamEvent>>,
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.clone());
        self.background.emit(event);
    }
}

/// @step Given a background session has post-tool hooks recording tool names and inputs
/// @step And its committed batch contains distinct function calls and a namespaced custom patch call
/// @step When I interrupt its streaming consumer during the first tool
/// @step Then each cancellation hook receives its own tool name and normalized input exactly once
/// @step And persisted conversation blocks contain every committed call before its outcome
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn cancellation_hooks_receive_the_matching_committed_tool_metadata() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let manifest = create_session_with_provider("hook pairs", directory.path(), "codex")?;
    let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
    let mut session = Session::from_provider_manager(manager);
    session.bind_provider_persistence(manifest.id)?;
    let batch = committed_interruption::Batch::new(&session, 0).await?;
    let hook_path = directory.path().join("hook-payloads");
    std::fs::create_dir(&hook_path)?;
    let command = format!("cat > '{}/hook-'\"$$\"", hook_path.display());
    let hooks = CompiledLifecycleHooks {
        global_timeout: 5,
        global_shell: None,
        session_start: Vec::new(),
        session_end: Vec::new(),
        user_prompt_submit: Vec::new(),
        notification: Vec::new(),
        pre_tool_use: Vec::new(),
        post_tool_use: vec![CompiledHookGroup {
            matcher: HookMatcher::Any,
            commands: vec![CompiledHookCommand {
                command,
                timeout: 5,
            }],
        }],
    };
    let background = Arc::new(BackgroundSession::new(
        manifest.id,
        "hook pairs".into(),
        directory.path().display().to_string(),
        Some("codex".into()),
        Some("shared-model".into()),
        Session::from_provider_manager(ProviderManager::for_testing(
            ProviderType::Codex,
            Some(272_000),
            Some(4096),
        )),
        mpsc::channel(1).0,
        None,
        None,
        Some(Arc::new(hooks)),
        broadcast::channel(32).0,
        broadcast::channel(32).0,
        broadcast::channel(32).0,
    ));
    let output = RecordingOutput {
        background: BackgroundOutput::with_provider(background, "codex".into(), None),
        events: Mutex::new(Vec::new()),
    };
    persist_user_message(&manifest.id, "inspect")?;
    batch.run(&mut session, manifest.id, &output).await?;
    let payloads = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let payloads: Vec<Value> = std::fs::read_dir(&hook_path)?
                .filter_map(Result::ok)
                .filter_map(|entry| std::fs::read(entry.path()).ok())
                .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
                .collect();
            if payloads.len() == 3 {
                return Ok::<_, std::io::Error>(payloads);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    let mut actual: Vec<_> = payloads
        .iter()
        .map(|payload| {
            assert!(payload["tool_response"]
                .as_str()
                .is_some_and(|text| text.contains("cancelled")));
            (
                payload["tool_name"].as_str().unwrap_or_default(),
                payload["tool_input"].clone(),
            )
        })
        .collect();
    actual.sort_by_key(|(name, _)| *name);
    let mut expected: Vec<_> = (0..3).map(committed_interruption::expected).collect();
    expected.sort_by_key(|(name, _)| *name);
    assert_eq!(
        actual, expected,
        "cancellation hooks must not reuse the first tool metadata"
    );
    committed_interruption::assert_pairs(&output.events.lock().map_err(|_| "events poisoned")?, 0);
    let blocks: Vec<Value> = get_session_message_envelopes(manifest.id)?
        .iter()
        .map(|encoded| serde_json::from_str::<Value>(encoded))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|envelope| {
            envelope["message"]["content"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|block| matches!(block["type"].as_str(), Some("tool_use" | "tool_result")))
        .collect();
    assert_eq!(blocks.len(), 6);
    for (index, pair) in blocks.chunks_exact(2).enumerate() {
        assert_eq!(pair[0]["type"], "tool_use");
        assert_eq!(pair[0]["id"], format!("item-{index}"));
        assert_eq!(pair[1]["tool_use_id"], pair[0]["id"]);
        assert_eq!(pair[0]["call_id"], format!("call-{index}"));
        assert_eq!(pair[1]["call_id"], pair[0]["call_id"]);
        assert_eq!(pair[1]["is_error"], true);
        let (name, input) = committed_interruption::expected(index);
        assert_eq!(pair[0]["name"], name);
        assert_eq!(pair[0]["input"], input);
    }
    Ok(())
}
