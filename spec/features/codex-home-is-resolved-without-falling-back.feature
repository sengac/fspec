@wip
@BUG-658
@providers
@authentication
@oauth
@rust
@bug-fix
Feature: Codex home is resolved without falling back
  """
  rust/providers/src/codex/auth_lifecycle.rs normalized_home canonicalizes the Codex home so pending logins are keyed by one path for every alias. It used canonicalize().unwrap_or_else(|_| home.to_path_buf()), so a home that could not be resolved was silently keyed by the unresolved path (no-fallback rule). It now returns AuthError::Home naming the path and the cause.
  cancel_at (sign-out, delete, credential write and cancel_login) used to run when the home might not exist yet. Its callers now create the home first (create_dir_all, as the auth lock does) and then cancel. Cancellation never waits on the auth lock: sign-out and delete cancel before they take it and cancel_login never takes it, so a login whose persistence holds or awaits the lock is cancelled at once (a first version cancelled under the lock and deadlocked prov159 A10). A login begun earlier can never commit afterwards: its commit re-checks the cancel flag and the generation under the lock.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. normalized_home returns the canonicalize error naming the Codex home path; it never falls back to the unresolved path (no-fallback rule)
  #   2. cancel_at runs only once the Codex home exists: the caller creates the home first (create_dir_all, as the auth lock does), so sign-out, delete and cancel_login still work when there is no Codex home (supervisor, 2026-11-09)
  #   3. Cancellation never waits on the auth lock: sign-out and delete cancel pending logins before they take the lock, and cancel_login never takes it, so a login whose persistence holds or awaits the lock is cancelled at once; a login begun before sign-out or delete can never commit afterwards, and a login begun after it can (supervisor, 2026-11-09, after the prov159 A10 deadlock)
  #
  # EXAMPLES:
  #   1. CODEX_HOME points at a directory that does not exist: complete_device_login fails with an error that names that path instead of reporting the login as changed
  #   2. CODEX_HOME points at a directory that does not exist: clear_codex_oauth, delete_codex_auth and cancel_login succeed and the home now exists
  #   3. A login begun before clear_codex_oauth cannot commit afterwards and no tokens are stored; a login begun after it commits
  #   4. A login begun before delete_codex_auth cannot commit afterwards and no tokens are stored
  #   5. Another process holds the auth lock while a login's commit waits for it: cancel_login returns at once, the commit fails as changed and no tokens are stored
  #
  # ========================================
  Background: User Story
    As a developer signing in to Codex through fspec
    I want a Codex home that cannot be resolved to be reported as an exact error naming the path
    So that pending logins are never matched or cancelled under a guessed path

  Scenario: An unresolvable Codex home is an error naming the path
    Given CODEX_HOME points at a directory that does not exist
    When a device login is completed
    Then it fails with an error that names the Codex home path
    And it is not reported as a changed login

  Scenario: Sign-out, delete and cancel work when there is no Codex home
    Given CODEX_HOME points at a directory that does not exist
    When the Codex OAuth tokens are cleared, the credentials are deleted and the login is cancelled
    Then each succeeds
    And the Codex home now exists

  Scenario: A login begun before sign-out cannot commit after it
    Given a Codex login attempt begun before sign-out
    When the user signs out and the attempt then commits its tokens
    Then the commit fails and no tokens are stored
    And a login begun after the sign-out commits its tokens

  Scenario: A login begun before delete cannot commit after it
    Given a Codex login attempt begun before the credentials are deleted
    When the credentials are deleted and the attempt then commits its tokens
    Then the commit fails and no tokens are stored

  Scenario: Cancelling a login does not wait for the auth lock
    Given a Codex login attempt
    And the Codex auth lock is held by another holder
    And the attempt is committing its tokens and waits for the lock
    When the login is cancelled
    Then the cancellation returns while the lock is still held
    And the commit fails as a changed login
    And no tokens are stored once the lock is released
