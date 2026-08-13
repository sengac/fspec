//! Stage Permissions Matcher
//!
//! Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
//!
//! Glob pattern matching for file categorization and stage permission checking.

use super::config::{FileCategory, StagePermissionsConfig};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashMap;

/// Result of checking a file write against stage permissions
#[derive(Debug, Clone)]
pub struct StageCheckResult {
    /// Whether the write is allowed
    pub allowed: bool,
    /// Whether the write is blocked
    pub blocked: bool,
    /// The file category matched (if any)
    pub matched_category: Option<String>,
    /// The current stage
    pub stage: String,
    /// Reason for blocking (if blocked)
    pub reason: Option<String>,
    /// Guidance message (if blocked)
    pub guidance: Option<String>,
}

impl StageCheckResult {
    /// Create an allowed result
    pub fn allowed(category: Option<String>, stage: String) -> Self {
        Self {
            allowed: true,
            blocked: false,
            matched_category: category,
            stage,
            reason: None,
            guidance: None,
        }
    }

    /// Create a blocked result
    pub fn blocked(category: String, stage: String, reason: String, guidance: String) -> Self {
        Self {
            allowed: false,
            blocked: true,
            matched_category: Some(category),
            stage,
            reason: Some(reason),
            guidance: Some(guidance),
        }
    }
}

/// Compiled glob patterns for a category
struct CompiledCategory {
    name: String,
    include: GlobSet,
    exclude: GlobSet,
}

/// Matcher for stage permissions
pub struct StagePermissionsMatcher {
    /// Compiled file categories
    categories: Vec<CompiledCategory>,
    /// Stage to writable categories mapping
    stage_permissions: HashMap<String, Vec<String>>,
}

impl StagePermissionsMatcher {
    /// Create a new matcher from configuration
    pub fn new(config: StagePermissionsConfig) -> Self {
        // Compile category glob patterns
        let categories: Vec<CompiledCategory> = config
            .categories
            .into_iter()
            .map(Self::compile_category)
            .collect();

        // Build stage permissions map
        let mut stage_permissions = HashMap::new();
        for perm in config.permissions {
            stage_permissions.insert(perm.stage, perm.writable_categories);
        }

        Self {
            categories,
            stage_permissions,
        }
    }

    /// Compile a category's glob patterns
    fn compile_category(category: FileCategory) -> CompiledCategory {
        let mut include_builder = GlobSetBuilder::new();
        let mut exclude_builder = GlobSetBuilder::new();

        for pattern in &category.patterns {
            if let Some(negated) = pattern.strip_prefix('!') {
                // Negation pattern - add to exclude set
                if let Ok(glob) = Glob::new(negated) {
                    exclude_builder.add(glob);
                }
            } else {
                // Include pattern
                if let Ok(glob) = Glob::new(pattern) {
                    include_builder.add(glob);
                }
            }
        }

        CompiledCategory {
            name: category.name,
            include: include_builder.build().unwrap_or_else(|_| GlobSet::empty()),
            exclude: exclude_builder.build().unwrap_or_else(|_| GlobSet::empty()),
        }
    }

    /// Check which category a file belongs to
    pub fn categorize_file(&self, path: &str) -> Option<String> {
        // Normalize path separators for cross-platform matching
        let normalized_path = path.replace('\\', "/");

        for category in &self.categories {
            // Check if file matches any include pattern
            if category.include.is_match(&normalized_path) {
                // Check if file is excluded by any negation pattern
                if !category.exclude.is_match(&normalized_path) {
                    return Some(category.name.clone());
                }
            }
        }
        None
    }

    /// Check if a file write is allowed for the given stage
    pub fn check_write(&self, path: &str, stage: &str) -> StageCheckResult {
        // If stage has no permissions defined, allow all writes
        let writable = match self.stage_permissions.get(stage) {
            Some(categories) => categories,
            None => return StageCheckResult::allowed(None, stage.to_string()),
        };

        // Categorize the file
        let category = match self.categorize_file(path) {
            Some(cat) => cat,
            // File doesn't match any category - allow by default
            None => return StageCheckResult::allowed(None, stage.to_string()),
        };

        // Check if the category is writable in this stage
        if writable.contains(&category) {
            StageCheckResult::allowed(Some(category), stage.to_string())
        } else {
            let (reason, guidance) = Self::get_block_message(stage, &category);
            StageCheckResult::blocked(category, stage.to_string(), reason, guidance)
        }
    }

    /// Get appropriate block message based on stage and category
    fn get_block_message(stage: &str, category: &str) -> (String, String) {
        match stage {
            "testing" => (
                format!(
                    "Blocked: Cannot write {category} files in testing stage. Write tests first, then move to implementing stage."
                ),
                "Use: fspec update-work-unit-status <id> implementing".to_string(),
            ),
            "specifying" => (
                format!(
                    "Blocked: Cannot write {category} files in specifying stage. Complete the specification first."
                ),
                "Use: fspec update-work-unit-status <id> testing (after generating scenarios)"
                    .to_string(),
            ),
            "validating" => (
                "Blocked: Cannot write files in validating stage. No modifications allowed during validation.".to_string(),
                "If changes needed, go back to implementing: fspec update-work-unit-status <id> implementing".to_string(),
            ),
            "done" => (
                "Blocked: Cannot write files for completed work unit. Work is marked as done.".to_string(),
                "To make changes, reopen the work unit or create a new one.".to_string(),
            ),
            _ => (
                format!("Blocked: Cannot write {category} files in {stage} stage."),
                format!("Check the stage permissions for {stage} stage."),
            ),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn create_test_config() -> StagePermissionsConfig {
        StagePermissionsConfig::default_acdd()
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Block implementation file write in testing stage
    #[test]
    fn test_block_implementation_file_write_in_testing_stage() {
        // @step Given the current work unit is in "testing" stage
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "testing";

        // @step And the project defines "impl" category as "src/**/*.ts" excluding test files
        // (defined in default_acdd config)

        // @step And "testing" stage only allows writing to "spec" and "test" categories
        // (defined in default_acdd config)

        // @step When the AI tries to write to "src/auth.ts"
        let result = matcher.check_write("src/auth.ts", stage);

        // @step Then the write should be blocked
        assert!(result.blocked);
        assert!(!result.allowed);

        // @step And the AI should receive error "Blocked: Cannot write implementation files in testing stage. Write tests first, then move to implementing stage."
        assert!(result
            .reason
            .as_ref()
            .expect("reason should be set")
            .contains("testing stage"));
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Allow test file write in testing stage
    #[test]
    fn test_allow_test_file_write_in_testing_stage() {
        // @step Given the current work unit is in "testing" stage
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "testing";

        // @step And the project defines "test" category as "src/**/*.test.ts" and "src/**/__tests__/**"
        // (defined in default_acdd config)

        // @step And "testing" stage allows writing to "spec" and "test" categories
        // (defined in default_acdd config)

        // @step When the AI tries to write to "src/__tests__/auth.test.ts"
        let result = matcher.check_write("src/__tests__/auth.test.ts", stage);

        // @step Then the write should be allowed
        assert!(result.allowed);
        assert!(!result.blocked);

        // @step And the file should be written successfully
        assert!(result.matched_category.is_some());
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Block code file write in specifying stage
    #[test]
    fn test_block_code_file_write_in_specifying_stage() {
        // @step Given the current work unit is in "specifying" stage
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "specifying";

        // @step And "specifying" stage only allows writing to "spec" category
        // (defined in default_acdd config)

        // @step When the AI tries to write to "src/feature.ts"
        let result = matcher.check_write("src/feature.ts", stage);

        // @step Then the write should be blocked
        assert!(result.blocked);
        assert!(!result.allowed);

        // @step And the AI should receive error "Blocked: Cannot write code files in specifying stage. Complete the specification first."
        assert!(result
            .reason
            .as_ref()
            .expect("reason should be set")
            .contains("specifying stage"));
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Allow attachment in backlog stage
    #[test]
    fn test_allow_attachment_in_backlog_stage() {
        // @step Given the current work unit is in "backlog" stage
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "backlog";

        // @step And "backlog" stage allows writing to "spec" category
        // (defined in default_acdd config)

        // @step When the AI tries to write to "spec/attachments/WORK-001/diagram.png"
        let result = matcher.check_write("spec/attachments/WORK-001/diagram.png", stage);

        // @step Then the write should be allowed
        assert!(result.allowed);
        assert!(!result.blocked);
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Block all writes in validating stage
    #[test]
    fn test_block_all_writes_in_validating_stage() {
        // @step Given the current work unit is in "validating" stage
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "validating";

        // @step And "validating" stage allows writing to nothing
        // (defined in default_acdd config)

        // @step When the AI tries to write to "src/auth.ts"
        let result = matcher.check_write("src/auth.ts", stage);

        // @step Then the write should be blocked
        assert!(result.blocked);
        assert!(!result.allowed);

        // @step And the AI should receive error containing "go back to implementing"
        assert!(result
            .guidance
            .as_ref()
            .expect("guidance should be set")
            .contains("go back to implementing"));
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Block all writes in done stage
    #[test]
    fn test_block_all_writes_in_done_stage() {
        // @step Given the current work unit is in "done" stage
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "done";

        // @step And "done" stage allows writing to nothing
        // (defined in default_acdd config)

        // @step When the AI tries to write to "spec/features/test.feature"
        let result = matcher.check_write("spec/features/test.feature", stage);

        // @step Then the write should be blocked
        assert!(result.blocked);
        assert!(!result.allowed);

        // @step And the AI should receive error containing "reopen"
        assert!(result
            .guidance
            .as_ref()
            .expect("guidance should be set")
            .contains("reopen"));
    }

    /// Feature: spec/features/stage-permissions-acdd-file-write-enforcement.feature
    /// Scenario: Allow all writes when no work unit is active
    #[test]
    fn test_allow_all_writes_when_no_work_unit_is_active() {
        // @step Given no work unit is set for the session
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);

        // Stage is empty/unknown when no work unit is active
        let stage = "";

        // @step When the AI tries to write to "src/auth.ts"
        let result = matcher.check_write("src/auth.ts", stage);

        // @step Then the write should be allowed
        assert!(result.allowed);
        assert!(!result.blocked);

        // @step And no ACDD stage enforcement should occur
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_categorize_file() {
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);

        // Spec files
        assert_eq!(
            matcher.categorize_file("spec/features/login.feature"),
            Some("spec".to_string())
        );

        // Test files
        assert_eq!(
            matcher.categorize_file("src/__tests__/auth.test.ts"),
            Some("test".to_string())
        );
        assert_eq!(
            matcher.categorize_file("src/utils.test.ts"),
            Some("test".to_string())
        );

        // Implementation files (should NOT match if we have proper exclusion)
        // Note: The impl pattern includes *.ts but excludes *.test.ts
        assert_eq!(
            matcher.categorize_file("src/auth.ts"),
            Some("impl".to_string())
        );
    }

    #[test]
    fn test_implementing_stage_allows_all() {
        let config = create_test_config();
        let matcher = StagePermissionsMatcher::new(config);
        let stage = "implementing";

        // All file types should be allowed in implementing stage
        assert!(matcher.check_write("src/auth.ts", stage).allowed);
        assert!(
            matcher
                .check_write("src/__tests__/auth.test.ts", stage)
                .allowed
        );
        assert!(
            matcher
                .check_write("spec/features/login.feature", stage)
                .allowed
        );
    }
}
