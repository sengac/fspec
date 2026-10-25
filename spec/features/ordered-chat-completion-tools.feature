@done
@BUG-262
@providers
@provider-abstraction
Feature: Ordered Chat Completion Tools
  """
  Mechanically separate the existing Chat request, decoder and unit-test modules
  before changing ordering. Keep request serialization and terminal behavior.
  Accumulate indexed streaming calls in provider index order at both flush sites.
  Real loopback SSE and JSON requests feed actual native Done acceptance; earlier
  calls perform filesystem side effects and later calls remain unexecuted.
  """

  Background: User Story
    As a user completing a task through Chat tools
    I want earlier work to execute before accepted completion
    So that Done cannot silently skip required side effects

  Scenario: Tool finish flush preserves provider index order before Done
    Given reverse arriving fragmented indexed work and native Done followed by trailing work
    When a loopback Chat stream finishes with tool_calls
    Then earlier work executes in index order with intact identities and arguments before accepted Done cancels trailing work

  Scenario: EOF flush preserves provider index order before Done
    Given reverse arriving fragmented indexed work and native Done followed by trailing work
    When a loopback Chat stream finishes with stop and reaches EOF
    Then earlier work executes in index order with intact identities and arguments before accepted Done cancels trailing work

  Scenario: JSON response vector retains streaming ordering parity
    Given indexed work and native Done followed by trailing work in a JSON response vector
    When a loopback nonstreaming Chat request is adapted to a stream
    Then earlier work executes in index order with intact identities and arguments before accepted Done cancels trailing work
