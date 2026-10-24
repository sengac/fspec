@done
@session
@compaction-resume
@phase-1
@CMPCT-060
Feature: Typed durable summary identity
  Persisted envelope identity distinguishes durable summaries from replaceable reminders.

  """
  _compactionSummary true identifies a durable DAG. A versioned _runtimeReminder
  identity explicitly designates a replaceable reminder. Marker-bearing text is
  ordinary content unless its envelope has a recognized structural identity.
  Identity validation precedes any mutation of generic or provider replay state.
  """

  Scenario: A marker wrapped durable summary survives cold restore
    Given a persisted DAG summary containing system reminder markers
    When cold restore prepares the next model history
    Then the exact durable summary occurs once in model history

  Scenario: Quoted and split markers remain legitimate user content
    Given string and split text user envelopes quoting reminder markers
    When their persisted history is restored
    Then every quoted marker and user text remains in model history

  Scenario: Reminder text cannot discard a mixed tool result envelope
    Given a persisted user envelope with quoted reminder text and a tool result
    When its history is restored
    Then its text remains in model history and its tool result remains in UI replay

  Scenario: Only explicitly replaceable runtime reminders are omitted
    Given a version one replaceable runtime reminder envelope
    When its history is restored
    Then that reminder is omitted without removing surrounding durable content

  Scenario: Incompatible reminder identity fails before restore mutation
    Given an envelope with ambiguous or incompatible typed reminder identity
    When its history is restored
    Then restore fails explicitly without installing partial model history
