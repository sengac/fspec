//! BUG-318: profile-qualified model strings resolve to their provider.
//!
//! Feature: spec/features/profile-composite-model-provider.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "profile_session_support/mod.rs"]
mod profile_session_support;

use std::sync::Mutex;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::model_parsing::parse_model_string;
use profile_session_support::{manager_with_seeded_cache, restore_user_dir, seed_qwen_profile};

static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

#[test]
fn a_profile_qualified_model_string_parses_to_its_provider() -> Result<(), String> {
    // @step Given the model string "openai:qwen/Qwen3-80B"
    let model = "openai:qwen/Qwen3-80B";

    // @step When parse_model_string is called
    let parsed = parse_model_string(model)?;

    // @step Then it returns registry_provider "openai" and model_part "Qwen3-80B"
    assert_eq!(parsed.registry_provider, "openai");
    assert_eq!(parsed.model_part, "Qwen3-80B");

    // @step And is_profile_model is true
    assert!(parsed.is_profile_model);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_created_from_a_profile_qualified_default_reports_the_provider(
) -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the global default model is "openai:qwen/Qwen3-80B" with a seeded qwen profile
    let (data_dir, manager, saved) = manager_with_seeded_cache()?;
    seed_qwen_profile(data_dir.path());
    manager.set_default_model("openai:qwen/Qwen3-80B")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When a new session is created
    let sid = handle.create_session_checked(None)?;
    assert!(!sid.value.is_empty(), "the session must be created");

    // @step Then the session model reports provider "openai" and model "Qwen3-80B"
    let model = handle
        .get_session_model(&sid)
        .expect("get_session_model reads the session");
    restore_user_dir(saved);
    assert_eq!(model.provider_id, "openai");
    assert_eq!(model.model_id, "Qwen3-80B");
    Ok(())
}
