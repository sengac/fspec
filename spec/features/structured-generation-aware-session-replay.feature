@done
@session
@compaction-resume
@phase-1
@CMPCT-061
Feature: Structured generation aware session replay
  Cold replay preserves model content independently of UI presentation.

  """
  Supported text, signed thinking, tool calls and tool results retain their
  original order and identity. Unsupported blocks fail before either provider
  or generic history mutation. A replay snapshot carries its session and
  committed manifest revision. Installing an older revision is an explicit
  conflict, not an archive fallback. Reinstallation replaces the previous
  replay rather than appending it, retaining fresh runtime context separately,
  including runtime messages added after the previous replay installation.
  """

  Scenario: Structured assistant and tool history survives on the next wire
    Given persisted assistant strings signed thinking tool calls and tool results
    When cold restore prepares the next provider request
    Then the request preserves text thinking signature call identity and results

  Scenario: Repeated restore replaces rather than duplicates history
    Given a populated session with a committed summary and retained suffix
    When the same committed history is restored twice
    Then one summary and one suffix remain with unchanged runtime context

  Scenario: Unsupported replay blocks fail before any history mutation
    Given a valid prefix followed by an unsupported persisted content block
    When structured restore is attempted
    Then restore fails without installing the prefix or inventing replacement content

  Scenario: A stale replay revision cannot replace a newer committed history
    Given replay envelopes captured before a newer message commit
    When the older snapshot is restored
    Then restore reports a revision conflict and retains the installed history

  Scenario: Refreshing runtime prefix reminders does not corrupt a replay installation
    Given an installed replay with a trusted committed revision
    And runtime context is refreshed before a new persisted suffix arrives
    When the current committed history is restored
    Then the exact installed replay is located without losing or duplicating runtime context

  Scenario: Ambiguous replay relocation fails without dropping content
    Given an installed replay that appears twice after runtime prefix changes
    When the current committed history is restored
    Then restore reports an explicit conflict and retains all history without fallback

  Scenario: Gemini transport decoration does not change durable replay identity
    Given committed tool history without a provider thought signature
    When Gemini adds its recognized transport-only placeholder before sending
    Then repeated replay retains the same durable history identity
    And genuine signature or tool argument changes remain explicit conflicts
