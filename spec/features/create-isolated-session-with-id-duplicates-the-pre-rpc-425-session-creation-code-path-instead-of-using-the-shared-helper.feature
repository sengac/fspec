@done
@session-management
@git-integration
@integration
@wt-012
@session
@bug-fix
@rust
@source-shape
@WT-012
Feature: create_isolated_session_with_id duplicates the pre-RPC-425 session-creation code path instead of using the shared helper

  """
  Design: add resolve_provider_manager(model: &str) -> Result<ProviderManager, String> in rust/sessions/src/model_resolution.rs funneling the three construction branches (registry: with_model_support + select_model; profile: with_provider_and_model + apply_profile_env_vars; codex/custom: with_provider_and_model). All three creation methods call it after credential resolution. create_isolated_session_with_id then builds SessionCreationParams (worktree_path, base_commit, Some(IsolationContext)) and delegates to create_background_session_inner; it keeps worktree creation, git-session manifest write, best-effort persistence manifest, IsolationStateChange emission (WT-001 3-arg), worktree-cwd footer poller, and the IsolatedSessionInfo return. Non-isolated callers gain the shared funnel (selection branch moves into the helper, PM construction stays at the call site).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A shared resolve_provider_manager(model: &str) -> Result<ProviderManager, String> helper (sessions crate) builds the provider manager for ALL model kinds — registry (with_model_support + select_model), profile (with_provider_and_model + apply_profile_env_vars bridge), codex and custom (with_provider_and_model) — so all three creation paths (create_session_with_id, create_session_from_manifest, create_isolated_session_with_id) share one construction+selection funnel and cannot drift; provider manager resolution must happen AFTER credential resolution (resolve_and_set_env_var) so the registry re-detects freshly set credentials
  #   2. create_isolated_session_with_id must delegate its shared bootstrap steps (model selection, vision/max-images seeding, Session construction + PROV-143 preserve-thinking seed, context-reminder injection, lifecycle-hook load, BackgroundSession construction, thinking-level seed, PROV-142 auto-continue seed, model limits, pre-tool hook registration, MCP init) to create_background_session_inner via SessionCreationParams carrying worktree_path, base_commit and the IsolationContext — isolation-specific work (create_worktree, git-session manifest write, best-effort session manifest persist, IsolationStateChange emission, footer poller with worktree cwd, IsolatedSessionInfo return) stays in the manager method
  #   3. create_session_with_id and create_session_from_manifest must keep their existing observable behavior after the migration: registry models via with_model_support+select_model (profile/codex/custom via with_provider_and_model), the shared helper for the bootstrap, manifest persistence placement unchanged (with_id persists a fresh manifest before the session map insert; from_manifest never writes a manifest), set_default_model + maybe_start_scheduler still only on those two paths
  #   4. An isolated session created against a profile model (provider:profile/model) must seed preserve_thinking_enabled from the profile's stored preserveThinking default (absent ⇒ false) and auto-continue from the profile's stored autoContinue value (≥1 ⇒ on with that budget, 0/absent ⇒ off with the default budget of 10) — identical seeding to non-isolated profile-model sessions, via the shared helper
  #
  # EXAMPLES:
  #   1. A git repo with one commit; creating an isolated session with a registry model (anthropic/claude-opus-4-5) yields IsolatedSessionInfo{worktree_path=<repo>/.fspec/worktrees/<id>, base_commit=<HEAD sha>}; the BackgroundSession carries worktree_path + base_commit, the isolation context reminder is injected, the session manifest is persisted (best-effort), the IsolationStateChange(true, worktree, base) chunk is broadcast, and the footer poller runs with the worktree cwd — exactly as before the refactor
  #   2. A stored profile 'spark' (autoContinue 300, preserveThinking false) against model openai:spark/o3: an ISOLATED session created via create_isolated_session_with_id seeds auto-continue ON with budget 300 and preserve_thinking_enabled=false — the same seeds a non-isolated profile-model session receives from the shared helper (the two known drifts, PROV-142/PROV-143, are closed for isolated sessions)
  #   3. Source shape: the create_isolated_session_with_id body calls create_background_session_inner and NO LONGER inlines BackgroundSession::new, load_lifecycle_hooks, register_pre_tool_hook, init_mcp_session, set_base_thinking_level, set_model_limits or set_session_model_vision — those live only in session_creation_helper.rs; the method still contains create_worktree, create_session_manifest, isolation_state_change_with_base and the worktree-cwd footer poller
  #   4. End-to-end hooks: a project whose spec/fspec-hooks.json defines a pre_tool_use command matching Bash and printing {"hookSpecificOutput":{"permissionDecision":"deny","reason":"no bash"}}: a tool call in an ISOLATED session is denied with the hook's reason — the pre-tool hook registered by the shared helper is live for isolated sessions
  #   5. Error path: create_isolated_session_with_id against a non-git directory fails with a 'Failed to create worktree' error before any session bootstrap runs, and calling it again for an already-created session id still fails with 'Session <id> already exists' (the manager prechecks stay in the method)
  #   6. Parity regression: a non-isolated profile-model session created via create_session_with_id against a profile with autoContinue 300 / preserveThinking true still seeds auto-continue ON budget 300 and preserve_thinking_enabled=true, and a registry-model session via create_session_with_id still succeeds with the with_model_support + select_model funnel (registry validation, credentials re-detection) — the shared provider-manager helper is behavior-preserving for the two migrated callers
  #
  # ========================================

  Background: User Story
    As a Rust developer
    I want to route create_isolated_session_with_id through the shared session-creation helper
    So that future session-bootstrap changes (PROV-*, HOOK-*, RPC-*) apply once and the isolated path cannot silently drift

  Scenario: An isolated session against a registry model preserves the full isolation surface
    Given a git repository with one committed file
    And a fresh session manager with an offline models registry seeded
    When I create an isolated session with id "abc123" and registry model "anthropic/claude-opus-4-5"
    Then the returned IsolatedSessionInfo has worktree_path "<repo>/.fspec/worktrees/abc123" and base_commit equal to the repo HEAD sha
    And the in-memory BackgroundSession carries worktree_path and base_commit
    And the isolation context reminder was injected for the session
    And the session persistence manifest was written to disk best-effort
    And an IsolationStateChange(true, worktree, base_commit) chunk was broadcast
    And the footer poller was spawned with the worktree cwd

  Scenario: An isolated session against a profile model seeds preserve-thinking and auto-continue from the profile
    Given a stored profile "spark" with autoContinue 300 and preserveThinking false
    And a fresh session manager with an offline models registry seeded
    When I create an isolated session against model "openai:spark/o3"
    Then the session's auto-continue is enabled with budget 300
    And the session's preserve_thinking_enabled flag is false
    And these seeds match what a non-isolated session against the same profile receives

  Scenario: create_isolated_session_with_id delegates shared bootstrap to the shared helper
    Given the session manager source in rust/sessions/src/session_manager.rs
    When I inspect the create_isolated_session_with_id method body
    Then the body calls create_background_session_inner
    And the body no longer inlines BackgroundSession::new, load_lifecycle_hooks, register_pre_tool_hook, init_mcp_session, set_base_thinking_level, set_model_limits or set_session_model_vision
    And those shared steps live only in session_creation_helper.rs
    And the method still contains create_worktree, create_session_manifest, isolation_state_change_with_base and the worktree-cwd footer poller

  Scenario: A pre_tool_use lifecycle hook is live for an isolated session
    Given a project whose spec/fspec-hooks.json defines a pre_tool_use command matching "Bash" that prints {"reason":"no bash","hookSpecificOutput":{"permissionDecision":"deny"}}
    And an isolated session created for that project
    When a Bash tool call is checked for the isolated session
    Then the tool call is denied with the hook's reason "no bash"

  Scenario: Creating an isolated session fails cleanly on precheck errors
    Given a directory that is not a git repository
    When I call create_isolated_session_with_id for that directory
    Then the call fails with a "Failed to create worktree" error before any session bootstrap runs
    Given a git repository and a session id already present in the manager
    When I call create_isolated_session_with_id again with that id
    Then the call fails with "Session <id> already exists"

  Scenario: The shared provider-manager helper is behavior-preserving for non-isolated callers
    Given a stored profile "spark" with autoContinue 300 and preserveThinking true
    And a fresh session manager with an offline models registry seeded
    When I create a non-isolated session via create_session_with_id against model "openai:spark/o3"
    Then the session's auto-continue is enabled with budget 300
    And the session's preserve_thinking_enabled flag is true
    Given a registry model "anthropic/claude-opus-4-5"
    When I create a non-isolated session via create_session_with_id against that registry model
    Then the session is created via the with_model_support plus select_model funnel and succeeds
