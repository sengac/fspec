@done
@CMPCT-071
@CMPCT-083
@providers
@context-management
@compaction
@phase-1
Feature: Stub provider physical serializer parity
  """
  The explicit test-support adapter serializes its entire request deterministically.
  Admission and adapter invocation share the serializer. An opt-in stub_tool_call
  additional parameter produces one real tool submission, then ordinary output
  after its recorded outcome. Default stub behavior remains unchanged.
  """

  Scenario: Stub preparation preserves complete context without invoking the adapter
    Given a stub request with mandatory instructions tools and structured history
    When the selected adapter prepares and streams the request
    Then prepared bytes preserve every request field and match the invoked adapter bytes

  Scenario: Explicit stub tool submission uses the real streaming tool path
    Given a stub request explicitly configuring a tool submission
    When the streaming adapter consumes the request and its recorded tool outcome
    Then it emits the configured tool call exactly once and ordinary output afterward
