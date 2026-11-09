// Feature: spec/features/conversational-test-and-quality-check-tool-detection.feature
//
// The agent doc fspec init writes (spec/AGENTS.md for claude) has its
// <test-command> and <quality-check-commands> placeholders replaced from
// spec/fspec-config.json (CONFIG-002; split out of init/tests.rs by COV-071).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use tempfile::TempDir;

use super::registry::get_agent_by_id;
use super::templates::generate_agent_doc;
use super::{HomeEnv, HomePlatform};

/// No home directory: the user config never contributes, whatever the
/// machine's real home holds (BUG-533).
fn no_home() -> HomeEnv {
    HomeEnv::from_vars(HomePlatform::current(), |_| None)
}

#[test]
fn tool_commands_substituted_from_project_config() {
    // @step Given spec/fspec-config.json has tools.test.command = 'npm test' and tools.qualityCheck.commands = ['npm run lint', 'npm run fmt']
    let tmp = TempDir::new().unwrap();
    let spec = tmp.path().join("spec");
    std::fs::create_dir_all(&spec).unwrap();
    std::fs::write(
        spec.join("fspec-config.json"),
        r#"{ "tools": { "test": { "command": "npm test" },
             "qualityCheck": { "commands": ["npm run lint", "npm run fmt"] } } }"#,
    )
    .unwrap();

    // @step When fspec init generates spec/AGENTS.md for the claude agent
    let claude = get_agent_by_id("claude").unwrap();
    assert_eq!(claude.doc_template, "AGENTS.md");
    let doc = generate_agent_doc(claude, tmp.path(), &no_home()).unwrap();

    // @step Then the generated spec/AGENTS.md file should have all <test-command> placeholders replaced with 'npm test'
    assert!(!doc.contains("<test-command>"));
    assert!(doc.contains("npm test"));

    // @step And the generated spec/AGENTS.md file should have all <quality-check-commands> placeholders replaced with 'npm run lint && npm run fmt'
    assert!(!doc.contains("<quality-check-commands>"));
    assert!(doc.contains("npm run lint && npm run fmt"));
}
