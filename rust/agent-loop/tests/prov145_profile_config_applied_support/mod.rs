//! Shared fixtures for the PROV-145 tests (`prov145_profile_config_applied.rs`).
//! A sub-directory module under `tests/` is not its own test binary, so the
//! feature keeps one test file.

use std::path::PathBuf;
use std::sync::Arc;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::StreamOutput;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::session_manager::SessionManager;

/// PROV-132/cont009 precedent: serialize tests that swap the process-global
/// data directory.
pub static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Trimmed offline models.dev catalog (openai only). MODEL-011: o3 carries
/// its models.dev reasoning_options, so a session on it takes low, medium
/// and high. The same fixture as
/// rust/fspec-tui/tests/fixtures/prov101_models.json — inlined here so the
/// agent-loop crate stays self-contained (no cross-crate fixture paths).
const MODELS_FIXTURE: &str = r#"{
  "openai": {
    "id": "openai",
    "name": "OpenAI",
    "env": ["OPENAI_API_KEY"],
    "models": {
      "o3": {
        "id": "o3",
        "name": "o3",
        "reasoning": true,
        "reasoning_options": [{"type": "effort", "values": ["low", "medium", "high"]}],
        "tool_call": true,
        "attachment": true,
        "temperature": false,
        "limit": { "context": 200000, "output": 100000 }
      }
    }
  }
}
"#;

/// Create a fresh BackgroundSession (Noop hooks — no agent loop spawned).
/// Mirrors rust/fspec-tui/tests/cont008_goal_back_sync_test.rs.
pub async fn fresh_background_session() -> (
    std::path::PathBuf,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
) {
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    let cache_dir = data_dir.as_path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models.json");
    // RPC-423 precedent (cont009): reset stores BEFORE setting data
    // directory so init_session_store() points at the fresh temp dir.
    codelet_core::persistence::reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    std::env::set_var("OPENAI_API_KEY", "prov145-fake-key");
    let manager = SessionManager::new();
    // BUG-437: sessions are created in the manager's start workspace.
    manager
        .set_default_project(data_dir.clone())
        .expect("set the start workspace");
    assert_eq!(manager.set_default_model("openai/o3"), Ok(()));
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist after create_session");
    (data_dir, manager, session)
}

/// The 10 normal (distinct) words of the loop stream's prefix.
const NORMAL_WORDS: [&str; 10] = [
    "the",
    "architecture",
    "of",
    "the",
    "streaming",
    "loop",
    "detector",
    "relies",
    "on",
    "word",
];

/// The looping phrase (3 words).
const LOOP_PHRASE: [&str; 3] = ["alpha", "beta", "gamma"];

/// Build the full deterministic loop stream: 10 normal words + 80 loop
/// words. After warning, enough new complete words remain to independently
/// trigger again and escalate to abort.
pub fn loop_stream_words() -> Vec<String> {
    let mut words: Vec<String> = NORMAL_WORDS.iter().map(|w| format!("{w} ")).collect();
    for i in 0..80 {
        words.push(format!("{} ", LOOP_PHRASE[i % LOOP_PHRASE.len()]));
    }
    words
}

/// Feed the loop stream one word per `StreamEvent::Text` delta through the
/// given BackgroundOutput.
pub fn feed_loop_stream(output: &BackgroundOutput, words: &[String]) {
    for word in words {
        output.emit(codelet_cli::interactive::StreamEvent::Text(word.clone()));
    }
}

pub fn read_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = rust/agent-loop; walk up two parents to repo root.
    // BUG-416: no relative-to-cwd fallback; a missing root is a test failure.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(root) = manifest.parent().and_then(|p| p.parent()) else {
        panic!(
            "{} has no repository root two levels up",
            manifest.display()
        );
    };
    let path = root.join(rel);
    // BUG-418: compose the module tree (agent_loop.rs + agent_loop/*.rs).
    codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}
