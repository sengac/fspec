@done
@BUG-463
@session
@persistence
@model-selection
@rust
Feature: Default model read failures are reported
  """
  rust/sessions/src/default_model_persistence.rs load_default_model_with_dir
  used read_to_string(..).ok()? and serde_json::from_str(..).ok()?, and
  load_default_model used get_data_dir().ok()?, so an unreadable or
  malformed default-model.json (or an unset data directory) silently became
  "no default model" — in the infallible SessionManager::new and in
  last-used model resolution.
  Now load_default_model(_with_dir) returns Result<Option<String>, String>:
  a missing file (or absent / empty model key) is None; a read or parse
  failure, or an unresolvable data directory, is an error naming it.
  SessionManager::new no longer reads disk; the daemon
  (fspec/src/common.rs) calls SessionManager::restore_persisted_default_model,
  which propagates the error. last-used model resolution propagates it too.
  Tests: rust/sessions/tests/bug463_default_model_read_failures_are_reported.rs.
  """

  Background: User Story
    As a user whose default-model.json is damaged
    I want to have an unreadable or malformed default model file reported naming the file
    So that a damaged file never silently drops my chosen default model

  Scenario: A malformed default model file is an error naming it
    Given a data directory whose default-model.json contains "{ not valid json"
    When the persisted default model is loaded from that data directory
    Then loading fails naming default-model.json

  Scenario: Restoring the default model into a session manager reports a malformed file
    Given the data directory's default-model.json contains "{ not valid json"
    When a session manager restores the persisted default model
    Then the restore fails naming default-model.json
    And the session manager has no default model
