# GIT-020: Code Smells and E2E Testing Requirements

## Critical Issue

**Isolated sessions are NOT properly blocking tool calls from accessing files outside the worktree (main repo).**

The existing implementation has code for path validation, but the tests are completely bogus - they use test doubles instead of testing the actual integration.

---

## Code Smells Identified

### 1. Tests Use Mocks Instead of Real E2E Integration

**File:** `codelet/tools/tests/isolated_file_operations_test.rs`

The test file creates a `TEST_EFFECTIVE_CWD_MAP` test double:

```rust
/// Thread-safe test double for effective_cwd callback
static TEST_EFFECTIVE_CWD_MAP: RwLock<Option<HashMap<String, PathBuf>>> = RwLock::new(None);

fn set_test_effective_cwd(session_id: Uuid, cwd: PathBuf) {
    // ... sets up test double
}

fn get_test_effective_cwd(session_id_str: String) -> Option<PathBuf> {
    // ... reads from test double
}
```

**The Problem:** These tests then use `fs::write` and `fs::read` directly, testing that basic filesystem operations work. They prove NOTHING about the actual tool wrapper isolation mechanism.

```rust
// What the bogus test does:
let effective_cwd = get_test_effective_cwd(session_id.to_string()).expect("effective_cwd not set");
let resolved_path = effective_cwd.join("src/new-file.ts");
fs::write(&resolved_path, "test content").expect("Failed to write file");

// This just tests fs::write works, NOT that the tool wrapper blocks access!
```

### 2. Real Code Path Never Tested

**The actual path validation code exists in:**
- `codelet/tools/src/facade/wrapper.rs` - `validate_and_resolve_path()`
- `codelet/napi/src/session_manager.rs` - `get_session_effective_cwd()` callback

**But no test verifies:**
1. Creating a real isolated session via `sessionManagerCreateIsolated` NAPI binding
2. Invoking real tools (Read, Write, Edit, Ls, Grep, Glob, AstGrep, AstGrepRefactor)
3. Verifying that access to main project paths is BLOCKED with an error

### 3. Critical Security Gap

The isolation mechanism is a security boundary. An isolated session should NOT be able to:
- Read files from `/project/src/` when worktree is at `/project/.fspec/worktrees/xyz/`
- Write files to `/project/src/`
- Use path traversal (`../../src/file.ts`) to escape
- Use symlinks pointing outside worktree

Without E2E tests, we cannot prove this security boundary works.

---

## Architecture: How Isolation SHOULD Work

```
┌─────────────────────────────────────────────────────────────────────┐
│  sessionManagerCreateIsolated(sessionId, model, project, name)      │
│                              │                                       │
│                              ▼                                       │
│  Creates BackgroundSession with worktree_path set                   │
│  Registers session in SessionManager                                 │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Tool invocation (e.g., Read tool with file_path="/project/src/x")  │
│                              │                                       │
│                              ▼                                       │
│  FileToolFacadeWrapper.call()                                        │
│    └── validate_and_resolve_path(session_id, file_path, "read")     │
│          └── get_effective_cwd(session_id)                          │
│                └── GET_EFFECTIVE_CWD_CALLBACK.get()                 │
│                      └── get_session_effective_cwd(session_id_str)  │
│                            └── session.effective_cwd()              │
│                                  └── Returns worktree_path          │
│                                                                      │
│  Path validation: Is "/project/src/x" within worktree?              │
│    - NO → Return Err(ToolError::Validation) + emit block notification│
│    - YES → Proceed with file operation                              │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Files:**
- `codelet/tools/src/facade/wrapper.rs:551` - `validate_and_resolve_path()`
- `codelet/tools/src/facade/wrapper.rs:569` - `validate_and_resolve_path_with_cwd()`
- `codelet/napi/src/session_manager.rs:5945` - `get_session_effective_cwd()` callback
- `codelet/napi/src/session_manager.rs:1067` - `BackgroundSession::effective_cwd()`

---

## What MUST Be Done

### Phase 1: Delete Bogus Tests

Delete `codelet/tools/tests/isolated_file_operations_test.rs` entirely. It:
- Uses test doubles instead of real callbacks
- Tests `fs::write`/`fs::read`, not tool wrappers
- Provides false confidence while proving nothing

### Phase 2: Create Proper E2E Tests

Create tests in the NAPI layer (where we have access to real session creation):

**Location:** `codelet/napi/tests/isolated_session_file_blocking_e2e.rs`

**Test Structure:**
```rust
#[test]
fn test_isolated_session_read_blocked_for_main_project() {
    // 1. Create temp git repo
    let temp_dir = create_git_repo_fixture();
    let project_root = temp_dir.path();
    
    // 2. Create file in main project
    fs::write(project_root.join("src/secret.ts"), "secret content");
    
    // 3. Create isolated session via REAL NAPI binding
    let session_id = Uuid::new_v4();
    let result = session_manager_create_isolated(
        session_id.to_string(),
        "anthropic/claude-sonnet",
        project_root.to_string_lossy(),
        "Test Session"
    ).unwrap();
    
    // 4. Invoke REAL Read tool
    let read_result = invoke_read_tool(
        session_id,
        project_root.join("src/secret.ts").to_string_lossy()
    );
    
    // 5. Assert BLOCKED
    assert!(read_result.is_err());
    assert!(read_result.unwrap_err().contains("outside isolated worktree"));
}
```

### Phase 3: Test All Tools

Create E2E tests for EVERY tool:
- Read tool - absolute path, relative path, path traversal, symlink
- Write tool - absolute path, relative path, path traversal
- Edit tool - absolute path
- Ls tool - absolute path, relative path
- Grep tool - path parameter
- Glob tool - path parameter
- AstGrep tool - path parameter
- AstGrepRefactor tool - source_file parameter

### Phase 4: Test Backward Compatibility

Verify non-isolated sessions still work:
```rust
#[test]
fn test_non_isolated_session_read_allowed_for_all_paths() {
    // Create session via sessionManagerCreateWithId (NOT isolated)
    // Verify Read tool works for /project/src/
}
```

### Phase 5: Create Reusable Test Fixtures

```rust
struct IsolatedSessionFixture {
    temp_dir: TempDir,
    session_id: Uuid,
    worktree_path: PathBuf,
    project_root: PathBuf,
}

impl IsolatedSessionFixture {
    fn new() -> Self { /* sets up git repo, creates isolated session */ }
    fn invoke_read(&self, path: &str) -> Result<String, String> { /* ... */ }
    fn invoke_write(&self, path: &str, content: &str) -> Result<(), String> { /* ... */ }
    // ... other tools
}
```

---

## Scenarios to Test

### Blocking Scenarios (MUST return error)

| Tool | Input Path | Expected Behavior |
|------|------------|-------------------|
| Read | `/project/src/main.ts` (absolute to main) | BLOCKED |
| Read | `../../src/main.ts` (path traversal) | BLOCKED |
| Read | `escape/secret.ts` (symlink to main) | BLOCKED |
| Write | `/project/src/new.ts` | BLOCKED |
| Edit | `/project/src/config.ts` | BLOCKED |
| Ls | `/project/src/` | BLOCKED |
| Grep | `pattern` with path `/project/` | BLOCKED |
| Glob | `**/*.ts` with path `/project/` | BLOCKED |
| AstGrep | pattern with path `/project/` | BLOCKED |
| AstGrepRefactor | source_file `/project/src/x.ts` | BLOCKED |

### Allowed Scenarios (MUST succeed)

| Tool | Input Path | Expected Behavior |
|------|------------|-------------------|
| Read | `src/app.ts` (relative) | ALLOWED - resolves to worktree |
| Read | `/project/.fspec/worktrees/xyz/src/app.ts` | ALLOWED - within worktree |
| Write | `src/new.ts` (relative) | ALLOWED |
| Ls | `src/` (relative) | ALLOWED |
| Grep | pattern with path `src/` | ALLOWED |
| Glob | `**/*.ts` with path `src/` | ALLOWED |

### Backward Compatibility Scenarios

| Session Type | Tool | Path | Expected |
|--------------|------|------|----------|
| Non-isolated | Read | `/project/src/main.ts` | ALLOWED |
| Non-isolated | Write | `/project/src/new.ts` | ALLOWED |

---

## Success Criteria

1. ✅ All bogus tests deleted
2. ✅ E2E tests pass for all blocking scenarios
3. ✅ E2E tests pass for all allowed scenarios
4. ✅ E2E tests pass for backward compatibility
5. ✅ No mocks, stubs, or test doubles used for isolation mechanism
6. ✅ Tests create real sessions via NAPI bindings
7. ✅ Tests invoke real tools, not simulations
