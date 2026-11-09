#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use tempfile::TempDir;

use super::registry::{get_agent_by_id, SlashFormat::Toml, AGENT_REGISTRY};
use super::templates::generate_agent_doc;
use super::{available_agents, detect_agents, HomeEnv, HomePlatform};

/// No home directory: the user config never contributes, whatever the
/// machine's real home holds (BUG-533).
fn no_home() -> HomeEnv {
    HomeEnv::from_vars(HomePlatform::current(), |_| None)
}

#[test]
fn registry_contains_claude_and_gemini() {
    assert!(get_agent_by_id("claude").is_some());
    assert_eq!(
        get_agent_by_id("gemini").unwrap().slash_command_format,
        Toml
    );
    assert!(get_agent_by_id("bogus").is_none());
}

#[test]
fn available_agents_lists_all_registry_agents_in_order() {
    let agents = available_agents();
    assert_eq!(agents.len(), AGENT_REGISTRY.len());
    assert_eq!(agents[0].id, "claude");
    assert_eq!(agents[0].name, "Claude Code");
}

#[test]
fn detect_agents_finds_an_agent_by_its_detection_path() {
    let tmp = TempDir::new().unwrap();
    // A `.cursor/` directory marks the cursor agent as already in use.
    std::fs::create_dir_all(tmp.path().join(".cursor")).unwrap();
    let detected = detect_agents(tmp.path());
    assert_eq!(detected, vec!["cursor".to_string()]);
}

#[test]
fn detect_agents_returns_empty_for_a_clean_project() {
    let tmp = TempDir::new().unwrap();
    assert!(detect_agents(tmp.path()).is_empty());
}

#[test]
fn placeholders_are_replaced_for_claude() {
    let tmp = TempDir::new().unwrap();
    let claude = get_agent_by_id("claude").unwrap();
    let doc = generate_agent_doc(claude, tmp.path(), &no_home()).unwrap();
    assert!(!doc.contains("{{AGENT_NAME}}"));
    assert!(!doc.contains("{{DOC_TEMPLATE}}"));
    assert!(doc.contains("Claude Code"));
}

#[test]
fn system_reminders_preserved_for_claude_stripped_for_gemini() {
    let tmp = TempDir::new().unwrap();
    let claude = get_agent_by_id("claude").unwrap();
    let gemini = get_agent_by_id("gemini").unwrap();
    assert!(generate_agent_doc(claude, tmp.path(), &no_home())
        .unwrap()
        .contains("<system-reminder>"));
    let g = generate_agent_doc(gemini, tmp.path(), &no_home()).unwrap();
    assert!(!g.contains("<system-reminder>"));
    assert!(g.contains("**IMPORTANT:**"));
}

#[test]
fn meta_cognitive_phrases_removed_for_gemini() {
    let tmp = TempDir::new().unwrap();
    let gemini = get_agent_by_id("gemini").unwrap();
    let g = generate_agent_doc(gemini, tmp.path(), &no_home()).unwrap();
    assert!(!g.to_lowercase().contains("ultrathink"));
    assert!(!g.to_lowercase().contains("deeply consider"));
}

#[test]
fn ide_agent_uses_emoji_important_prefix() {
    let tmp = TempDir::new().unwrap();
    let cursor = get_agent_by_id("cursor").unwrap();
    let doc = generate_agent_doc(cursor, tmp.path(), &no_home()).unwrap();
    assert!(!doc.contains("<system-reminder>"));
    assert!(doc.contains("**⚠️ IMPORTANT:**"));
}

#[test]
fn tool_command_placeholders_intact_without_config() {
    // An empty home sandbox, injected (BUG-533), so a developer ~/.fspec
    // config does not substitute the placeholders under test.
    let home = TempDir::new().unwrap();
    let sandbox = home.path().as_os_str().to_owned();
    let home = HomeEnv::from_vars(HomePlatform::current(), |_| Some(sandbox.clone()));

    let tmp = TempDir::new().unwrap();
    let claude = get_agent_by_id("claude").unwrap();
    let doc = generate_agent_doc(claude, tmp.path(), &home).unwrap();

    assert!(doc.contains("<test-command>"));
    assert!(doc.contains("<quality-check-commands>"));
}
