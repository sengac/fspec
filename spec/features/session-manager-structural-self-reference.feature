@done
@BUG-420
@session
@session-management
@rust
Feature: A session manager always owns its self-reference
  """
  SessionManager.self_weak was a OnceLock<Weak<SessionManager>> filled only by
  new_arc / init_self_weak, and create_session_with_id (manager_impl_3.rs),
  create_session_from_manifest (manager_impl_4.rs) and
  create_isolated_session_with_id (manager_impl_5.rs) stamped each session with
  self.self_weak.get().cloned().unwrap_or_default(), so a manager built with
  SessionManager::new() gave every session a dangling owning-manager Weak and
  init_self_weak discarded OnceLock::set's result.
  SessionManager::new() now returns Arc<SessionManager> built with
  Arc::new_cyclic, the field is a plain Weak, and new_arc, init_self_weak and
  the Default impl are removed, so there is no way to build a manager without
  its self-reference.
  Tests: rust/sessions/tests/bug420_session_manager_structural_self_reference.rs.
  """

  Background: User Story
    As a developer creating sessions through a SessionManager
    I want to have every session reach the manager that created it
    So that a manager built without its self-reference can never stamp a dangling owning-manager reference

  Scenario: A session created by a new manager reaches that manager
    Given a session manager built with SessionManager::new
    When it creates a session
    Then the session's owning manager is that same manager

  Scenario: A manager cannot be built without its self-reference
    Given the codelet-sessions session manager sources
    When they are scanned for an optional self-reference
    Then there is no init_self_weak, no new_arc and no defaulted self-reference stamp
