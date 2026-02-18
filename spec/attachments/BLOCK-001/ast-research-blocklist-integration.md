# AST Research: Blocklist System Integration Components

**Work Unit:** BLOCK-001  
**Date:** 2026-02-18  
**Purpose:** AST analysis of all blocklist system components for integration testing

---

## 1. Rust Blocklist Core Module

### Location: `codelet/tools/src/blocklist/`

#### Public Functions (middleware.rs)
```
pub fn system_config_path() -> Option<PathBuf>
pub fn project_config_path(project_root: &Path) -> PathBuf
pub fn load_blocklist_config(project_root: Option<&Path>) -> BlocklistConfig
pub fn check_bash_command(command: &str) -> Result<(), BlockedError>
pub fn check_command_raw(command: &str) -> CheckResult
pub fn reload_blocklist(project_root: Option<&Path>) -> BlocklistMatcher
pub fn is_session_allowed(pattern: &str) -> bool
pub fn init_blocklist(project_root: Option<&Path>)
pub fn allow_for_session(pattern: &str)
pub fn clear_session_allowances()
```

#### Public Structs
```rust
// middleware.rs:22
pub struct BlockedError {
    // Error returned when command is blocked
}

// matcher.rs:10
pub struct CheckResult {
    pub allowed: bool,
    pub blocked: bool,
    pub reason: Option<String>,
    pub guidance: Option<String>,
    pub matched_rule_id: Option<String>,
}

// matcher.rs:48
pub struct BlocklistMatcher {
    // Compiled regex matcher for rules
}

// config.rs:21
pub struct BlocklistRule {
    pub id: String,
    pub pattern: String,
    pub action: BlocklistAction,
    pub reason: String,
    pub guidance: Option<String>,
}

// config.rs:37
pub struct BlocklistConfig {
    pub version: String,
    pub rules: Vec<BlocklistRule>,
}
```

#### Public Enums
```rust
// config.rs:10
pub enum BlocklistAction {
    Block,
    Allow,
    Prompt,
}
```

#### Matcher Functions (matcher.rs)
```
pub fn allowed() -> Self
pub fn blocked(rule: &BlocklistRule) -> Self
pub fn new(config: BlocklistConfig) -> Self
pub fn check_command(&self, command: &str) -> CheckResult
pub fn has_rules(&self) -> bool
pub fn rule_count(&self) -> usize
```

#### Config Functions (config.rs)
```
pub fn empty() -> Self
pub fn merge(system: BlocklistConfig, project: BlocklistConfig) -> Self
pub fn load_from_file(path: &std::path::Path) -> Result<Self, std::io::Error>
pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error>
```

---

## 2. NAPI Bindings for Blocklist

### Location: `codelet/napi/src/blocklist.rs`

#### Exported Functions
```rust
#[napi] pub fn blocklist_init(project_root: Option<String>) -> Result<()>
#[napi] pub fn blocklist_load(project_root: Option<String>) -> Result<JsBlocklistConfig>
#[napi] pub fn blocklist_save(project_root: String, config: JsBlocklistConfig) -> Result<()>
#[napi] pub fn blocklist_check(command: String) -> Result<JsCheckResult>
#[napi] pub fn blocklist_system_path() -> Option<String>
#[napi] pub fn blocklist_project_path(project_root: String) -> String
#[napi] pub fn blocklist_allow_session(pattern: String) -> Result<()>
#[napi] pub fn blocklist_is_session_allowed(pattern: String) -> bool
#[napi] pub fn blocklist_clear_session_allowances() -> Result<()>
```

#### JS-Friendly Types
```rust
#[napi(object)]
pub struct JsBlocklistRule {
    pub id: String,
    pub pattern: String,
    pub action: String,  // "block" | "allow" | "prompt"
    pub reason: String,
    pub guidance: Option<String>,
}

#[napi(object)]
pub struct JsBlocklistConfig {
    pub version: String,
    pub rules: Vec<JsBlocklistRule>,
}

#[napi(object)]
pub struct JsCheckResult {
    pub allowed: bool,
    pub blocked: bool,
    pub reason: Option<String>,
    pub guidance: Option<String>,
    pub matched_rule_id: Option<String>,
}
```

---

## 3. Stage Permissions Module

### Location: `codelet/tools/src/stage_permissions/`

#### Public Functions (mod.rs)
```
pub fn system_config_path() -> Option<PathBuf>
pub fn project_config_path(project_root: &Path) -> PathBuf
pub fn load_stage_permissions_config(project_root: Option<&Path>) -> StagePermissionsConfig
pub fn check_write_permission(path: &str, stage: Option<&str>) -> Result<(), StageBlockedError>
pub fn check_write_raw(path: &str, stage: &str) -> StageCheckResult
```

#### Public Structs
```rust
// mod.rs:44
pub struct StageBlockedError {
    // Error returned when write is blocked by ACDD stage
}

// matcher.rs:13
pub struct StageCheckResult {
    pub allowed: bool,
    pub blocked: bool,
    pub category: Option<String>,
    pub stage: String,
    pub reason: Option<String>,
    pub guidance: Option<String>,
}

// matcher.rs:62
pub struct StagePermissionsMatcher {
    // Compiled pattern matcher for file categories
}

// config.rs:13
pub struct FileCategory {
    pub name: String,
    pub patterns: Vec<String>,
}

// config.rs:23
pub struct StagePermission {
    pub stage: String,
    pub writable: Vec<String>,
}

// config.rs:32
pub struct StagePermissionsConfig {
    pub version: String,
    pub categories: Vec<FileCategory>,
    pub permissions: Vec<StagePermission>,
}
```

#### Matcher Functions (matcher.rs)
```
pub fn allowed(category: Option<String>, stage: String) -> Self
pub fn blocked(category: String, stage: String, reason: String, guidance: String) -> Self
pub fn new(config: StagePermissionsConfig) -> Self
pub fn categorize_file(&self, path: &str) -> Option<String>
pub fn check_write(&self, path: &str, stage: &str) -> StageCheckResult
```

#### Config Functions (config.rs)
```
pub fn empty() -> Self
pub fn default_acdd() -> Self
pub fn load_from_file(path: &Path) -> Result<Self, std::io::Error>
pub fn merge(system: Self, project: Self) -> Self
pub fn get_writable_categories(&self, stage: &str) -> Vec<&str>
```

---

## 4. TUI Components

### Location: `src/tui/components/BlocklistListView.tsx`

#### Exported Interface
```typescript
export interface BlocklistRule {
  id: string;
  pattern: string;
  action: 'block' | 'allow' | 'prompt';
  reason: string;
  guidance?: string;
  source: 'system' | 'project';
}
```

#### Exported Component
```typescript
export function BlocklistListView({
  rules,
  disabledRules,
  terminalWidth,
  terminalHeight,
  onToggleRule,
  onClose,
}: BlocklistListViewProps): JSX.Element
```

---

## 5. Facade Tool Wrappers

### Location: `codelet/tools/src/facade/wrapper.rs`

#### Tool Implementations (impl Tool for X)
```rust
impl Tool for FacadeToolWrapper           // Generic wrapper
impl Tool for FileToolFacadeWrapper       // Read/Write/Edit tools
impl Tool for FspecToolFacadeWrapper      // Fspec tool
impl Tool for BashToolFacadeWrapper       // Bash tool (blocklist checks here)
impl Tool for SearchToolFacadeWrapper     // Grep/Glob tools
impl Tool for LsToolFacadeWrapper         // Directory listing
impl Tool for BridgeToolFacadeWrapper     // Bridge tool
```

---

## 6. Existing Integration Tests

### Location: `codelet/tools/tests/block_notifications_integration_test.rs`

#### Test Functions
```rust
fn test_notification_callback_can_be_registered()
fn test_stage_callback_can_be_registered()
fn test_blocklist_check_returns_blocked_result()
fn test_blocklist_check_allows_unmatched_commands()
fn test_stage_permissions_block_impl_files_in_testing_stage()
fn test_stage_permissions_allow_test_files_in_testing_stage()
fn test_stage_permissions_allow_spec_files_in_testing_stage()
fn test_no_block_when_no_work_unit_active()
fn test_blocked_error_contains_reason_for_notification()
fn test_stage_permission_error_format()
```

### Location: `codelet/tools/tests/block_notifications_test.rs`

#### Test Functions
```rust
fn test_notify_user_when_ai_command_is_blocked()
fn test_notify_user_when_ai_file_write_is_blocked_by_stage_permissions()
fn test_test_files_not_blocked_in_testing_stage()
fn test_no_block_when_no_work_unit_active()
fn test_block_notification_message_format()
```

### Location: `src/__tests__/blocklist-napi-integration.test.ts`

#### Test Scenarios
- Block dangerous command with guidance
- Block Bash usage for file reading with tool guidance
- Project config overrides system config
- Blocklist path functions work correctly
- Load and save config persistence
- Allow unmatched commands
- Prompt action type
- Sensitive Path Prompts - Session Allowances NAPI Integration

---

## 7. Key Integration Points for Testing

### Config Hierarchy Flow
```
System Config (~/.fspec/blocklist.json)
        ↓
Project Config (.fspec/blocklist.json)
        ↓
BlocklistConfig.merge() → BlocklistMatcher
        ↓
Session Allowances (in-memory HashSet)
```

### Command Check Flow
```
AI Agent → FacadeToolWrapper → check_bash_command()
        ↓
BlocklistMatcher.check_command()
        ↓
CheckResult { allowed, blocked, reason, guidance, matched_rule_id }
        ↓
If blocked → BlockedError → Notification to TUI
```

### Stage Permission Flow
```
AI Agent → FileToolFacadeWrapper → check_write_permission()
        ↓
StagePermissionsMatcher.check_write(path, stage)
        ↓
StageCheckResult { allowed, blocked, category, stage, reason, guidance }
        ↓
If blocked → StageBlockedError → Notification to TUI
```

### TUI Integration Flow
```
User runs /blocklist
        ↓
blocklist_load() via NAPI → JsBlocklistConfig
        ↓
BlocklistListView renders rules
        ↓
User toggles rule → disabledRules Set updated
        ↓
blocklist_allow_session(pattern) for session-only override
```

### Session Memory Flow
```
User selects "Allow Session" in prompt dialog
        ↓
blocklist_allow_session(pattern) → SESSION_ALLOWANCES HashSet
        ↓
Subsequent checks: is_session_allowed(pattern) → true
        ↓
TUI restart: blocklist_clear_session_allowances() → HashSet cleared
```

---

## 8. Test Coverage Gaps Identified

Based on AST analysis, the following integration scenarios need coverage:

1. **Config Hierarchy with Session Override** - Full E2E from system rule → project override → session toggle
2. **Stage Permissions E2E** - Work unit state transitions with file write attempts
3. **Sensitive Path Prompts E2E** - Prompt dialog → Allow Session → subsequent access
4. **TUI Blocklist View E2E** - Full keyboard navigation and rule toggle persistence

These map directly to the 4 scenarios in the feature file.
