@done
@BUG-363
@tools
@tool-execution
@rust
Feature: Unified exec process store reports kill and wait failures
  """
  rust/tools/src/unified_exec: close_owned_session, close_session and
  evict_lru_if_full discarded child.kill() results (let _ =), try_wait mapped
  an error to Some(None) (still running) and eviction mapped a try_wait error
  to "not exited". ProcessStore::try_wait now returns
  Result<Option<Option<ExitStatus>>, String> naming the session; close_session
  returns a failed kill as Err; the owned close and eviction, which have no
  result to return, log failures at error level with the session id; poll
  returns a try_wait failure to the model as a tool error and the reaper and
  exec-stdin detector log it. Tests:
  rust/tools/tests/bug363_process_store_errors.rs.
  """

  Background: User Story
    As an agent running long-lived commands through unified exec
    I want a failed kill or status check reported
    So that a process that could not be stopped or checked is never shown as handled

  Scenario: Process store kill and wait failures are never discarded
    Given the unified exec process store sources
    When their child kill and try_wait calls are listed
    Then no kill result is discarded
    And no try_wait error is read as a running or exited process

  Scenario: try_wait distinguishes a running session from an unknown one
    Given a unified exec session whose command is still running
    When the store checks the session with try_wait
    Then it reports the session as running without an exit status
    And an unknown session reports no session
