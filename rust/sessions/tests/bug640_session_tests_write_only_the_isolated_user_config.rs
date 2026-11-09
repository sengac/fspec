//! Feature: spec/features/fspec-config-writes-are-atomic-and-tests-never-touch-the-real-user-config.feature
//!
//! BUG-640: a session test isolates its user directory through
//! `codelet_test_helpers::isolated_user_directory()` (the rpc410 support
//! does, so its default-model save lands in the isolated config), and the
//! sessions config writers are one locked read-modify-write each, so
//! concurrent custom-model saves keep every model.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "rpc410_hitl_wire_protocol_parity_support/mod.rs"]
#[allow(dead_code)]
mod rpc410_hitl_wire_protocol_parity_support;

#[path = "fixtures/offline_catalog.rs"]
#[allow(dead_code)]
mod offline_catalog;

use std::sync::{Arc, Barrier};

use codelet_sessions::profile_sections::{save_custom_model, CustomModelDef};
use codelet_sessions::SessionManager;
use serde_json::Value;

fn user_config() -> Value {
    let directory = codelet_test_helpers::isolated_user_directory().expect("isolated user dir");
    let raw = std::fs::read_to_string(directory.join("fspec-config.json")).expect("user config");
    serde_json::from_str(&raw).expect("the user config is valid JSON")
}

fn real_user_dir() -> std::path::PathBuf {
    dirs::home_dir().expect("a home directory").join(".fspec")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setting_a_default_model_in_a_session_test_writes_only_the_isolated_user_config() {
    // @step Given the rpc410_hitl_wire_protocol_parity test support with its isolated user directory
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step When a fresh session sets the default model "anthropic/claude-opus-4-5"
    rpc410_hitl_wire_protocol_parity_support::fresh_session(&manager).await;

    // @step Then the isolated user directory's fspec-config.json holds that model
    assert_eq!(
        user_config()["tui"]["lastUsedModel"],
        Value::from("anthropic/claude-opus-4-5")
    );

    // @step And the resolved user directory is not <home>/.fspec
    let resolved = codelet_common::user_dir::fspec_user_dir().expect("the isolated user dir");
    assert_eq!(
        resolved.as_path(),
        codelet_test_helpers::isolated_user_directory().expect("isolated user dir")
    );
    assert_ne!(resolved, real_user_dir());
}

#[test]
fn concurrent_writers_to_the_user_config_keep_every_change() {
    // @step Given a user fspec-config.json
    let directory = codelet_test_helpers::isolated_user_directory().expect("isolated user dir");
    codelet_common::fspec_config::update_config_file(
        &directory.join("fspec-config.json"),
        |root| {
            root["providers"]["openai"]["profiles"]["bug640"] =
                serde_json::json!({"baseUrl": "http://127.0.0.1:1"});
            Ok::<_, String>(())
        },
    )
    .expect("seed the profile");
    assert!(directory.join("fspec-config.json").is_file());

    // @step When 8 threads each save a different custom model to it at once
    let start = Arc::new(Barrier::new(8));
    let writers: Vec<_> = (0..8)
        .map(|index| {
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                let model: CustomModelDef =
                    serde_json::from_value(serde_json::json!({"id": format!("model-{index}")}))
                        .expect("a custom model");
                start.wait();
                save_custom_model("openai", "bug640", &model, None)
            })
        })
        .collect();
    for writer in writers {
        writer
            .join()
            .expect("the writer thread finished")
            .expect("the custom model was saved");
    }

    // @step Then the file is valid JSON
    let config = user_config();

    // @step And it holds all 8 custom models
    let mut ids: Vec<String> = config["providers"]["openai"]["profiles"]["bug640"]["customModels"]
        .as_array()
        .expect("customModels is an array")
        .iter()
        .map(|model| model["id"].as_str().expect("an id").to_string())
        .collect();
    ids.sort();
    let expected: Vec<String> = (0..8).map(|index| format!("model-{index}")).collect();
    assert_eq!(ids, expected);
}
