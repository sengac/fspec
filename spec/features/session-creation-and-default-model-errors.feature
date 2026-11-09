@done
@BUG-411
@session
@session-creation
@rust
Feature: Session creation and default-model changes report their failures
  """
  create_session_with_id (rust/sessions/src/session_manager/manager_impl_3.rs)
  logged a resolve_and_set_env_var failure with tracing::error and created the
  session anyway, and SessionManager::set_default_model (manager_impl_1.rs)
  persisted best-effort, only warning when save_default_model or
  save_persisted_model_string failed. Credential resolution returns Err only
  for real failures (a missing credential is Ok(false)), so creation now
  returns that error. set_default_model returns Result: the in-memory default
  is still updated, and a persistence failure is returned to the caller
  (SessionManagerHandle::set_default_model and the RPC propagate it).
  Tests: rust/sessions/tests/bug411_session_creation_and_default_model_errors.rs.
  """

  Background: User Story
    As a user creating sessions and choosing a default model
    I want to see a credential failure or a failed default-model save as an error
    So that a session is never created with credentials that silently failed to load and a model choice is never lost without notice

  Scenario: A credential resolution failure fails session creation
    Given a provider whose stored credentials cannot be read
    When a session is created for that provider
    Then creation fails with the credential error

  Scenario: A default model that cannot be persisted is reported
    Given a data directory the default model cannot be written into
    When the default model is set
    Then the call fails with the persistence error
