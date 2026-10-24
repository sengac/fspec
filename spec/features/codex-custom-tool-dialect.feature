@PROV-157
@providers
@provider-abstraction
@streaming
@integration
@wip
Feature: Codex custom tool dialect
  Users can explicitly enable raw-input tools without weakening existing execution controls.

  """
  Codex owns an opt-in dialect registry. Only apply_patch supports raw input initially,
  mapped to the existing executor's patch field. Namespace aliases resolve only to
  registered function tools. Completed calls use ordinary rig dispatch and hooks;
  custom results retain their original call IDs and protocol type in continuation.
  Unknown or disabled dialects fail closed. Generic providers remain unchanged.
  """

  Scenario: Registered raw patch input retains its grammar and text
    Given an enabled namespaced raw patch tool with a grammar
    When Codex registers the tool and completes a Unicode patch call
    Then the wire definition retains its grammar and the existing executor receives the exact patch

  Scenario: Namespace identity survives independent local tool names
    Given two namespaces exposing the same local function name
    When completed calls arrive for both namespaces
    Then each call selects its registered executor and keeps its result identity

  Scenario: Unregistered or malformed dialects cannot execute
    Given disabled custom tools or an invalid namespace registration
    When a custom call or invalid configuration is supplied
    Then an actionable error occurs before local tool execution

  Scenario: Custom results use the matching protocol result type
    Given a completed custom call and its local execution result
    When the next Codex request is serialized
    Then the raw call and custom result retain the original call identifier

  Scenario: Disabled dialect preserves ordinary tool registration
    Given an ordinary Codex or non-Codex function tool configuration
    When its request is serialized without dialect opt-in
    Then the ordinary function definition and arguments remain unchanged
