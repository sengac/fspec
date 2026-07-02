@done
@session-management
@agent-core
@rpc-087
@source-shape
@regression
@RPC-087
@rust
@agent-loop
Feature: Agent loop: error classification + retry (recovery_network/compaction/thinking/truncation/stall/image)
  """
  Regression-shape coverage card following the established
  RPC-149/150/151/152/153/155/156 pattern. Implementation pre-exists in
  codelet/cli/src/interactive/ (1922 LoC across recovery_*.rs +
  error_classifiers.rs + stream_loop.rs invocations). Tests assert
  source-string substrings + direct invocation of public surface API.

  Sibling deferred behavioural feature lives in
  spec/features/agent-loop-error-recovery.feature (@deferred). This
  feature pins the structural wiring that the behavioural feature would
  exercise at runtime.
  """

  Background: User Story
    As a fspec maintainer
    I want the error classification + recovery helper wiring in stream_loop.rs to be regression-shape pinned
    So that no one can silently drop the classifier or recovery_* call sites that keep transient errors recoverable instead of fatal

  Scenario: interactive/mod.rs declares all six recovery modules and the error_classifiers module
    Given the source file codelet/cli/src/interactive/mod.rs
    When I read the file as a string
    Then the body contains the substring "mod error_classifiers;"
    And the body contains the substring "mod recovery_compaction;"
    And the body contains the substring "mod recovery_image;"
    And the body contains the substring "mod recovery_network;"
    And the body contains the substring "mod recovery_stall;"
    And the body contains the substring "mod recovery_thinking;"
    And the body contains the substring "mod recovery_truncation;"

  Scenario: interactive crate re-exports the recovery + classifier public surface
    Given the source file codelet/cli/src/interactive/mod.rs
    When I read the file as a string
    Then the body contains the substring "pub use error_classifiers::{"
    And the body contains the substring "is_transient_network_error"
    And the body contains the substring "is_stall_timeout_error"
    And the body contains the substring "classify_compaction_branch"
    And the body contains the substring "pub use recovery_network::{"
    And the body contains the substring "MAX_NETWORK_RETRIES"
    And the body contains the substring "network_retry_delay"
    And the body contains the substring "pub use recovery_image::sanitize_image_content;"
    And the body contains the substring "STALL_TIMEOUT_ERROR_PREFIX"
    And the body contains the substring "MAX_TRUNCATION_RETRIES"

  Scenario: MAX_NETWORK_RETRIES constant is reachable via the public surface and equals 3
    Given the re-exported constant codelet_cli::interactive::MAX_NETWORK_RETRIES
    When I read its value
    Then it equals 3

  Scenario: network_retry_delay implements exponential backoff with 1s base
    Given the re-exported function codelet_cli::interactive::network_retry_delay
    When I call it with attempt 1
    Then it returns Duration::from_millis(1000)
    When I call it with attempt 2
    Then it returns Duration::from_millis(2000)
    When I call it with attempt 3
    Then it returns Duration::from_millis(4000)
    When I call it with attempt 0
    Then it returns Duration::from_millis(1000)

  Scenario: is_transient_network_error recognises common HTTP/connection failures while stall classifier does not
    Given the re-exported predicate codelet_cli::interactive::is_transient_network_error
    When I call it with "error sending request for url"
    Then it returns true
    When I call it with "connection reset by peer"
    Then it returns true
    When I call it with "connection refused"
    Then it returns true
    When I call it with "operation timed out"
    Then it returns true
    And calling codelet_cli::interactive::is_stall_timeout_error with "connection reset by peer" returns false

  Scenario: is_stall_timeout_error uses STALL_TIMEOUT_ERROR_PREFIX as single source of truth
    Given the re-exported predicate codelet_cli::interactive::is_stall_timeout_error
    And the re-exported constant codelet_cli::interactive::STALL_TIMEOUT_ERROR_PREFIX
    When I call the predicate with the constant value
    Then it returns true
    And the source file codelet/cli/src/interactive/error_classifiers.rs body contains the substring "super::recovery_stall::STALL_TIMEOUT_ERROR_PREFIX"

  Scenario: stream_loop.rs wires every classifier predicate and has at least one call site each
    Given the source file codelet/cli/src/interactive/stream_loop.rs
    When I read the file as a string
    Then the body contains the substring "use super::error_classifiers::{"
    And the body contains the substring "is_stall_timeout_error("
    And the body contains the substring "is_prompt_too_long_error("
    And the body contains the substring "is_image_content_error("
    And the body contains the substring "is_truncated_tool_call_error("
    And the body contains the substring "is_transient_network_error("
    And the body contains the substring "classify_compaction_branch("

  Scenario: stream_loop.rs guards retry with MAX_NETWORK_RETRIES and uses network_retry_delay
    Given the source file codelet/cli/src/interactive/stream_loop.rs
    When I read the file as a string
    Then the body contains the substring "use super::recovery_network::{MAX_NETWORK_RETRIES, network_retry_delay}"
    And the body contains the substring "network_retry_count <= MAX_NETWORK_RETRIES"
    And the body contains the substring "network_retry_delay(network_retry_count)"

  Scenario: stream_loop.rs sanitises image content after classifying an image-content rejection
    Given the source file codelet/cli/src/interactive/stream_loop.rs
    When I read the file as a string
    Then the body contains the substring "sanitize_image_content(&mut session.messages)"
    And the body contains the substring "is_image_content_error(&error_str)"
    And the byte offset of "is_image_content_error(&error_str)" is less than the byte offset of "sanitize_image_content(&mut session.messages)"
