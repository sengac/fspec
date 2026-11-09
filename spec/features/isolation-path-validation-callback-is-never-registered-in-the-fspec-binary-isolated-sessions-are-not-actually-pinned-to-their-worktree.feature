@session-management
@done
@bug-fix
@session
@tools
@git-integration
@wt-004
Feature: Isolation path-validation callback is never registered in the fspec binary — isolated sessions are not actually pinned to their worktree
  """
  codelet-sessions gains a NAPI-free module session_tool_callbacks (sibling of footer_poller, same manager-slot pattern): register_manager(Arc<SessionManager>) stores the manager in a std::sync::Mutex<Option<Arc<SessionManager>>>; a private fn session_by_id() consults the registered manager and only that manager (BUG-398 removed the NAPI-era SessionManager::instance() fallback: with no manager registered the lookup logs an error and finds no session); pub fn isolation_context(session_id_str) -> Result<Option<codelet_tools::facade::IsolationContext>, String> (Ok(Some) only for isolated sessions: worktree_path from session.worktree_path, blocked_project_path from session.project; BUG-404: a malformed id, an unknown session or a missing manager is Err, and the tool layer refuses the file operation), pub fn work_unit_stage(session_id_str) -> Option<String> (session.get_work_unit_context().and_then(|c| c.status)), and pub fn emit_block_notification(session_id_str, action, reason) (UserNotification(Warning) chunk "AI was blocked from {action} - {reason}" on the footer_poller chunk-sender slot; BUG-405: with no sender registered the notification is reported as undeliverable, never sent to a global). All three are plain fn pointers matching codelet-tools' callback types (BUG-404 changed GetEffectiveCwdCallback to return Result<Option<IsolationContext>, String>).
  build_service (rust/fspec/src/common.rs) registers the tool callbacks immediately after manager construction + set_hooks + the WT-002 register_chunk_sender call: codelet_sessions::session_tool_callbacks::register_manager(manager.clone()); codelet_tools::facade::set_get_effective_cwd_callback(codelet_sessions::session_tool_callbacks::isolation_context); codelet_tools::facade::set_get_work_unit_stage_callback(codelet_sessions::session_tool_callbacks::work_unit_stage); codelet_tools::facade::set_block_notification_callback(codelet_sessions::session_tool_callbacks::emit_block_notification). build_service returns before any session exists, so registration ordering vs session creation is safe; the OnceLocks accept the first set only — build_service is the only caller in the binary's process.
  NAPI parity (rust/napi/src/bridges.rs): init_block_notification_callbacks keeps its name and signature (the RPC-043 napi shape test pins it) but registers the SHARED codelet-sessions functions: the work-unit-stage and effective-cwd OnceLocks point directly at the shared work_unit_stage / isolation_context functions (the local get_session_work_unit_stage / get_session_effective_cwd duplicates are deleted); the block-notification OnceLock keeps the RPC-043-pinned name emit_block_notification_to_tui, which is now a one-line delegating shim over codelet_sessions::session_tool_callbacks::emit_block_notification (no local chunk-building logic left). NAPI does NOT call register_manager — the shared callbacks' manager slot stays empty and lookups fall back to SessionManager::instance(), which IS the NAPI session store, preserving today's behavior byte-for-byte. Only the fspec binary's build_service (non-singleton manager) calls register_manager.
  REFAC-053: the work-unit-stage-none, block-notification, build_service wiring and NAPI scenarios moved to wt004-tool-callbacks-wiring.feature (test file rust/sessions/tests/wt004_tool_callbacks_wiring.rs); this feature keeps the isolation-lookup scenarios in wt004_session_tool_callbacks.rs, with shared helpers in rust/sessions/tests/wt004_session_tool_callbacks_support/mod.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The isolation-context callback must be resolvable from the non-singleton SessionManager that a front door owns: when codelet-sessions::session_tool_callbacks::register_manager(manager) has been called, codelet_tools::get_isolation_context(session_id) looks up the session in THAT manager only; when unregistered no session is found (BUG-398: the NAPI-era SessionManager::instance() fallback is removed).
  #   2. codelet-sessions MUST expose the isolation lookup as a plain function pointer (fn(String) -> Option<codelet_tools::facade::IsolationContext>) so it can be stored in codelet-tools' OnceLock without changing the existing callback type; no OnceLock/OnceCell may be added to codelet-tools for this purpose.
  #   3. The fspec binary's build_service MUST register its non-singleton SessionManager as the tool-callbacks manager (codelet_sessions::session_tool_callbacks::register_manager(manager)) immediately after constructing the manager and installing hooks — BEFORE any session can be created — so tool path isolation, stage-gated writes, and block notifications are live from the first session.
  #   4. Isolation context resolution semantics (unchanged): for isolated sessions Some(IsolationContext{worktree_path, blocked_project_path: session project root}); for non-isolated sessions, sessions not found in the manager, and the unregistered-manager case, None — tools degrade to today's allow-all behavior, never stricter.
  #   5. (Obsolete since the NAPI crate was removed, BUG-398) The NAPI front door must stay green: codelet-napi's init_block_notification_callbacks (and any napi shape tests asserting its shape) keep registering the shared codelet-sessions callback functions; the shared isolation lookup and block-notification emitters become the single source of truth used by BOTH front doors (two-front-doors rule), with NAPI registering SessionManager::instance() as its manager.
  #   6. The work-unit-stage callback (BLOCK-006 stage-gated writes) and the block-notification callback must also be live in the fspec binary: codelet-sessions owns a shared work-unit-stage lookup (session -> work unit context status) and a shared block-notification emitter that sends a UserNotification(Warning) chunk "AI was blocked from {action} - {reason}" on the registered manager's chunks_tx (register_chunk_sender slot; BUG-405: no global fallback); both are wired into codelet-tools' OnceLocks by the same registration step.
  #
  # EXAMPLES:
  #   1. In the fspec binary (combined mode), after build_service, an isolated session created against a temp git repo has get_isolation_context(session_id) == Some(ctx) where ctx.worktree_path == <repo>/.fspec/worktrees/<id>/ and ctx.blocked_project_path == the repo root.
  #   2. Under the fspec binary, an isolated session's Write tool targeting an absolute path inside the original project root fails with a validation error naming the blocked project path, while a Write to a relative path lands in the worktree.
  #   3. A non-isolated session created under the fspec binary still reads/writes anywhere (e.g. /tmp) exactly as before — isolation only applies to sessions that have a worktree.
  #
  # ========================================
  Background: User Story
    As a developer running the fspec binary (combined/daemon mode)
    I want to create an isolated (worktree) session and have every per-session tool honor that isolation
    So that the worktree is an actual boundary, not a cosmetic badge

  # ============================================================================
  # CORE FIX — the isolation callback resolves through the fspec binary's
  # non-singleton SessionManager (business rule 1, 3)
  # ============================================================================
  Scenario: Isolated session under the fspec binary exposes its isolation context to the tool layer
    Given a fresh git repository with one committed file
    And a non-singleton session manager whose tool callbacks are registered the way build_service registers them
    When an isolated session is created against that repository through the registered manager
    Then the tool layer resolves an isolation context for that session with the worktree path of the created worktree
    And the isolation context's blocked project path is the repository root

  Scenario: Isolation lookup through the registered manager ignores any other manager
    Given a non-singleton session manager registered as the tool-callbacks manager
    And an isolated session created in that manager against a fresh git repository
    When the tool layer resolves the isolation context for that session id
    Then the lookup succeeds even though a second, unregistered manager has no such session
    And the returned context matches the session's worktree path and project root

  # ============================================================================
  # BEHAVIORAL ENFORCEMENT — file tools honor the boundary under the binary
  # (business rule 4)
  # ============================================================================
  Scenario: Write tool of an isolated session blocks absolute paths into the original project
    Given a non-singleton session manager registered as the tool-callbacks manager
    And an isolated session created against a fresh git repository that contains a tracked source file
    When a file operation for that session validates an absolute path inside the original project root
    Then the validation fails with an error naming the blocked project path
    And the original project file is not modified

  Scenario: Write tool of an isolated session resolves relative paths into the worktree
    Given a non-singleton session manager registered as the tool-callbacks manager
    And an isolated session created against a fresh git repository
    When a file operation for that session validates a relative path
    Then the resolved path lands inside the session's worktree directory

  Scenario: Non-isolated session keeps unrestricted path access
    Given a non-singleton session manager registered as the tool-callbacks manager
    And a plain non-isolated session created in that manager
    When the tool layer resolves the isolation context for that session
    Then it gets no isolation context
    And a path validation for any absolute path outside the project succeeds unchanged

  Scenario: Unknown session ids refuse isolation lookups
    Given a non-singleton session manager registered as the tool-callbacks manager
    When the tool layer resolves the isolation context for a session id that no manager knows
    Then the lookup fails naming the session instead of returning no isolation context

  # ============================================================================
  # BLOCK-006 + notification parity — stage-gated writes and block
  # notifications are live in the fspec binary (business rule 6)
  # ============================================================================
  Scenario: Work unit stage lookup under the fspec binary reads the session's work unit context
    Given a non-singleton session manager registered as the tool-callbacks manager
    And a session with a work unit context whose status is testing
    When the tool layer asks for that session's work unit stage
    Then it returns testing
