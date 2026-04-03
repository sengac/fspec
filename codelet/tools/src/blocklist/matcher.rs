//! Blocklist Matcher - Regex-based command matching
//!
//! Evaluates commands against blocklist rules using regex patterns.

use regex::Regex;
use tracing::error;
use super::config::{BlocklistAction, BlocklistConfig, BlocklistRule};

/// Result of checking a command against the blocklist
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Whether the command is allowed to execute
    pub allowed: bool,
    /// Whether the command is blocked
    pub blocked: bool,
    /// Reason for blocking (if blocked)
    pub reason: Option<String>,
    /// Guidance on what to do instead (if blocked)
    pub guidance: Option<String>,
    /// ID of the matching rule (if any)
    pub matched_rule_id: Option<String>,
}

impl CheckResult {
    /// Create a result for an allowed command
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            blocked: false,
            reason: None,
            guidance: None,
            matched_rule_id: None,
        }
    }

    /// Create a result for a blocked command
    pub fn blocked(rule: &BlocklistRule) -> Self {
        Self {
            allowed: false,
            blocked: true,
            reason: Some(rule.reason.clone()),
            guidance: rule.guidance.clone(),
            matched_rule_id: Some(rule.id.clone()),
        }
    }
}

/// Compiled blocklist matcher for efficient rule evaluation
pub struct BlocklistMatcher {
    /// Compiled rules with regex patterns
    compiled_rules: Vec<CompiledRule>,
}

/// A rule with its compiled regex pattern
struct CompiledRule {
    rule: BlocklistRule,
    regex: Regex,
}

impl BlocklistMatcher {
    /// Create a new matcher from a blocklist config
    pub fn new(config: BlocklistConfig) -> Self {
        let compiled_rules = config
            .rules
            .into_iter()
            .filter_map(|rule| {
                match Regex::new(&rule.pattern) {
                    Ok(regex) => Some(CompiledRule { rule, regex }),
                    Err(e) => {
                        // Log invalid patterns but don't fail
                        error!("Warning: Invalid regex pattern '{}': {}", rule.pattern, e);
                        None
                    }
                }
            })
            .collect();

        Self { compiled_rules }
    }

    /// Check a command against all blocklist rules
    ///
    /// Rules are evaluated in order (project rules first if merged).
    /// The first matching rule determines the result.
    pub fn check_command(&self, command: &str) -> CheckResult {
        for compiled in &self.compiled_rules {
            if compiled.regex.is_match(command) {
                match compiled.rule.action {
                    BlocklistAction::Block => {
                        return CheckResult::blocked(&compiled.rule);
                    }
                    BlocklistAction::Allow => {
                        // Allow action means this specific pattern is explicitly allowed
                        // (used for project overrides)
                        return CheckResult::allowed();
                    }
                    BlocklistAction::Prompt => {
                        // Prompt action will be handled by BLOCK-005
                        // For now, treat as a special blocked case
                        return CheckResult {
                            allowed: false,
                            blocked: false, // Not blocked, but needs user confirmation
                            reason: Some(compiled.rule.reason.clone()),
                            guidance: compiled.rule.guidance.clone(),
                            matched_rule_id: Some(compiled.rule.id.clone()),
                        };
                    }
                }
            }
        }

        // No matching rule - command is allowed
        CheckResult::allowed()
    }

    /// Check if any rules exist
    pub fn has_rules(&self) -> bool {
        !self.compiled_rules.is_empty()
    }

    /// Get the number of compiled rules
    pub fn rule_count(&self) -> usize {
        self.compiled_rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block_rule(id: &str, pattern: &str, reason: &str) -> BlocklistRule {
        BlocklistRule {
            id: id.to_string(),
            pattern: pattern.to_string(),
            action: BlocklistAction::Block,
            reason: reason.to_string(),
            guidance: None,
        }
    }

    #[test]
    fn test_simple_pattern_matching() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![make_block_rule("test", "^rm\\s+-rf", "Dangerous command")],
        };
        let matcher = BlocklistMatcher::new(config);

        assert!(matcher.check_command("rm -rf /").blocked);
        assert!(matcher.check_command("rm -rf ./foo").blocked);
        assert!(matcher.check_command("echo rm -rf").allowed); // doesn't start with rm
        assert!(matcher.check_command("rm file.txt").allowed); // no -rf flag
    }

    #[test]
    fn test_word_boundary_pattern() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![make_block_rule("checkout", r"^git\s+checkout\b", "Use git switch")],
        };
        let matcher = BlocklistMatcher::new(config);

        assert!(matcher.check_command("git checkout main").blocked);
        assert!(matcher.check_command("git checkout -b feature").blocked);
        assert!(matcher.check_command("git checkouts").allowed); // "checkouts" != "checkout"
    }

    #[test]
    fn test_allow_override() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![
                // More specific allow rule first
                BlocklistRule {
                    id: "allow-node-modules".to_string(),
                    pattern: r"^rm\s+-rf\s+\./node_modules\b".to_string(),
                    action: BlocklistAction::Allow,
                    reason: "".to_string(),
                    guidance: None,
                },
                // General block rule second
                BlocklistRule {
                    id: "block-rm-rf".to_string(),
                    pattern: r"^rm\s+-rf\b".to_string(),
                    action: BlocklistAction::Block,
                    reason: "Dangerous".to_string(),
                    guidance: None,
                },
            ],
        };
        let matcher = BlocklistMatcher::new(config);

        // Specific pattern is allowed
        assert!(matcher.check_command("rm -rf ./node_modules").allowed);
        
        // General pattern is blocked
        assert!(matcher.check_command("rm -rf /").blocked);
        assert!(matcher.check_command("rm -rf ./src").blocked);
    }

    #[test]
    fn test_check_result_contains_rule_info() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![BlocklistRule {
                id: "test-rule-123".to_string(),
                pattern: "^test".to_string(),
                action: BlocklistAction::Block,
                reason: "Test reason".to_string(),
                guidance: Some("Test guidance".to_string()),
            }],
        };
        let matcher = BlocklistMatcher::new(config);

        let result = matcher.check_command("test command");
        
        assert!(result.blocked);
        assert_eq!(result.reason, Some("Test reason".to_string()));
        assert_eq!(result.guidance, Some("Test guidance".to_string()));
        assert_eq!(result.matched_rule_id, Some("test-rule-123".to_string()));
    }

    #[test]
    fn test_invalid_regex_is_skipped() {
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: vec![
                BlocklistRule {
                    id: "invalid".to_string(),
                    pattern: "[invalid".to_string(), // Invalid regex
                    action: BlocklistAction::Block,
                    reason: "".to_string(),
                    guidance: None,
                },
                make_block_rule("valid", "^test", "Valid rule"),
            ],
        };
        let matcher = BlocklistMatcher::new(config);

        // Invalid rule is skipped, valid rule still works
        assert_eq!(matcher.rule_count(), 1);
        assert!(matcher.check_command("test").blocked);
    }
}
