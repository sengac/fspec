//! Blocklist Module - Command and Tool Filtering System
//!
//! Feature: spec/features/blocklist-core-command-tool-filtering.feature
//!
//! This module implements command blocking with educational guidance for AI agents.
//! It supports blocking dangerous commands (e.g., git checkout) and wrong tool usage
//! (e.g., using Bash for file reading instead of the Read tool).
//!
//! ## Architecture
//!
//! - `config.rs`: BlocklistConfig and BlocklistRule types
//! - `matcher.rs`: BlocklistMatcher for regex evaluation
//! - `middleware.rs`: FilterMiddleware for tool execution interception
//!
//! ## Config Hierarchy
//!
//! 1. System config: `~/.fspec/blocklist.json`
//! 2. Project config: `.fspec/blocklist.json`
//! 3. Project rules extend/override system rules

mod config;
mod matcher;
mod middleware;

pub use config::{BlocklistAction, BlocklistConfig, BlocklistRule};
pub use matcher::{BlocklistMatcher, CheckResult};
pub use middleware::{
    allow_for_session, check_bash_command, check_command_raw, check_file_path,
    clear_session_allowances, init_blocklist, is_session_allowed, load_blocklist_config,
    project_config_path, reload_blocklist, system_config_path, BlockedError,
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Feature: spec/features/blocklist-core-command-tool-filtering.feature
    /// Scenario: Block dangerous command with guidance
    #[test]
    fn test_block_git_checkout_with_guidance() {
        // @step Given a blocklist rule exists blocking "git checkout" with reason "Use git switch instead"
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "git-checkout-block".to_string(),
                pattern: r"^git\s+checkout\b".to_string(),
                action: BlocklistAction::Block,
                reason: "git checkout is deprecated".to_string(),
                guidance: Some("Use git switch instead".to_string()),
            }],
        };
        let matcher = BlocklistMatcher::new(config);

        // @step When the AI runs "git checkout main" via Bash
        let command = "git checkout main";
        let result = matcher.check_command(command);

        // @step Then the command should be blocked
        assert!(result.blocked, "Command should be blocked");
        assert!(!result.allowed, "Command should not be allowed");

        // @step And the AI should receive error "Blocked: git checkout is deprecated. Use git switch main instead."
        assert!(
            result
                .reason
                .as_ref()
                .expect("reason should be set")
                .contains("git checkout is deprecated"),
            "Reason should mention git checkout is deprecated"
        );
        assert!(
            result
                .guidance
                .as_ref()
                .expect("guidance should be set")
                .contains("git switch"),
            "Guidance should mention git switch"
        );
    }

    /// Feature: spec/features/blocklist-core-command-tool-filtering.feature
    /// Scenario: Block Bash usage for file reading with tool guidance
    #[test]
    fn test_block_cat_command_with_read_tool_guidance() {
        // @step Given a blocklist rule exists blocking "cat" commands with reason "Use Read tool instead"
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "cat-block".to_string(),
                pattern: r"^cat\s+".to_string(),
                action: BlocklistAction::Block,
                reason: "Use the Read tool for file reading, not Bash".to_string(),
                guidance: Some("This ensures proper encoding and line number display.".to_string()),
            }],
        };
        let matcher = BlocklistMatcher::new(config);

        // @step When the AI runs "cat src/file.ts" via Bash
        let command = "cat src/file.ts";
        let result = matcher.check_command(command);

        // @step Then the command should be blocked
        assert!(result.blocked, "Command should be blocked");
        assert!(!result.allowed, "Command should not be allowed");

        // @step And the AI should receive error "Blocked: Use the Read tool for file reading, not Bash. This ensures proper encoding and line number display."
        assert!(
            result
                .reason
                .as_ref()
                .expect("reason should be set")
                .contains("Use the Read tool"),
            "Reason should mention Read tool"
        );
        assert!(
            result
                .guidance
                .as_ref()
                .expect("guidance should be set")
                .contains("proper encoding"),
            "Guidance should mention proper encoding"
        );
    }

    /// Feature: spec/features/blocklist-core-command-tool-filtering.feature
    /// Scenario: Project config overrides system config
    #[test]
    fn test_project_config_overrides_system_config() {
        // @step Given system blocklist at "~/.fspec/blocklist.json" blocks "rm -rf"
        let system_config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "rm-rf-block".to_string(),
                pattern: r"^rm\s+-rf\b".to_string(),
                action: BlocklistAction::Block,
                reason: "Dangerous command - rm -rf can delete everything".to_string(),
                guidance: None,
            }],
        };

        // @step And project blocklist at ".fspec/blocklist.json" allows "rm -rf ./node_modules"
        let project_config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "rm-rf-node-modules-allow".to_string(),
                pattern: r"^rm\s+-rf\s+\./node_modules\b".to_string(),
                action: BlocklistAction::Allow, // Project allows this specific pattern
                reason: "".to_string(),
                guidance: None,
            }],
        };

        // Merge configs: project rules take precedence (more specific patterns first)
        let merged_config = BlocklistConfig::merge(system_config, project_config);
        let matcher = BlocklistMatcher::new(merged_config);

        // @step When the AI runs "rm -rf ./node_modules"
        let allowed_command = "rm -rf ./node_modules";
        let allowed_result = matcher.check_command(allowed_command);

        // @step Then the command should be allowed
        assert!(
            allowed_result.allowed,
            "rm -rf ./node_modules should be allowed"
        );
        assert!(
            !allowed_result.blocked,
            "rm -rf ./node_modules should not be blocked"
        );

        // @step When the AI runs "rm -rf /"
        let blocked_command = "rm -rf /";
        let blocked_result = matcher.check_command(blocked_command);

        // @step Then the command should be blocked
        assert!(blocked_result.blocked, "rm -rf / should be blocked");
        assert!(!blocked_result.allowed, "rm -rf / should not be allowed");
    }

    #[test]
    fn test_regex_pattern_matching() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "git-checkout-block".to_string(),
                pattern: r"^git\s+checkout\b".to_string(),
                action: BlocklistAction::Block,
                reason: "Use git switch".to_string(),
                guidance: None,
            }],
        };
        let matcher = BlocklistMatcher::new(config);

        // Should match: starts with "git checkout"
        assert!(matcher.check_command("git checkout main").blocked);
        assert!(matcher.check_command("git checkout -b feature").blocked);

        // Should NOT match: doesn't start with "git checkout"
        assert!(matcher.check_command("echo git checkout").allowed);
        assert!(matcher.check_command("git switch main").allowed);
        assert!(matcher.check_command("git status").allowed);
    }

    #[test]
    fn test_allow_unmatched_commands() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "git-checkout-block".to_string(),
                pattern: r"^git\s+checkout\b".to_string(),
                action: BlocklistAction::Block,
                reason: "Use git switch".to_string(),
                guidance: None,
            }],
        };
        let matcher = BlocklistMatcher::new(config);

        // Unmatched commands should be allowed
        let result = matcher.check_command("npm install");
        assert!(result.allowed);
        assert!(!result.blocked);
    }
}
