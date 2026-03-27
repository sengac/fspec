@CMPCT-019
Feature: Incremental DAG Condensation
  """
  Only file modified: codelet/cli/src/interactive_helpers.rs — add detect_existing_dag(), split instruction constants, update execute_compaction()
  Reuses parse_dag_nodes from codelet-core (CMPCT-017) to extract max turn_end from existing DAG
  Tests in inject_summary_handler_test.rs that reference COMPACTION_SYSTEM_INSTRUCTION need updating to use new constant names
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. execute_compaction() must detect whether a compaction-dag system-reminder exists in current messages BEFORE clearing them
  #   2. When no existing DAG is found, COMPACTION_INSTRUCTION_FRESH is used (first-time compaction)
  #   3. When an existing DAG is found, COMPACTION_INSTRUCTION_INCREMENTAL is used with the existing DAG content embedded
  #   4. The incremental instruction must include the last compacted turn index derived from max turn_end of parsed DagNodeMeta
  #   5. The incremental instruction must tell the agent to: preserve D2 nodes, review D1 nodes, promote D0→D1, and create new D0 nodes for fresh turns only
  #   6. Detection searches for <!-- type:compaction-dag --> marker in user message content
  #   7. The FRESH instruction is the current COMPACTION_SYSTEM_INSTRUCTION content (no behavioral change for first-time compaction)
  #   8. Template substitution replaces {existing_dag_content} and {last_compacted_turn} placeholders in the incremental instruction
  #
  # EXAMPLES:
  #   1. First compaction: no existing DAG → FRESH instruction used → agent builds full DAG from SessionSearch
  #   2. Second compaction: existing DAG found with max turn_end=82 → INCREMENTAL instruction with existing_dag_content and start_turn=83
  #   3. detect_existing_dag finds DAG in messages list containing <!-- type:compaction-dag --> marker → returns (content, max_turn_end)
  #   4. detect_existing_dag on messages with no DAG → returns None
  #   5. Incremental template {existing_dag_content} is replaced with actual DAG content, {last_compacted_turn} is replaced with max turn_end
  #   6. execute_compaction with last_user_message appends resume prompt after both fresh and incremental instructions
  #   7. Existing DAG with no parseable dag-node blocks → fallback turn_end=0 so incremental starts from beginning
  #   8. COMPACTION_SYSTEM_INSTRUCTION constant is removed after split (replaced by FRESH and INCREMENTAL)
  #
  # ========================================
  Background: User Story
    As a compaction engine
    I want to use incremental DAG condensation when re-compacting a session that already has a DAG summary
    So that I avoid rebuilding the entire DAG from scratch each time and preserve established decisions

  Scenario: First compaction uses FRESH instruction when no existing DAG
    Given a session with conversation messages but no compaction-dag system-reminder
    When execute_compaction is called
    Then the injected user message should contain the FRESH compaction instruction
    And the FRESH instruction should mention SessionSearch for strategic searching
    And the FRESH instruction should explain D0, D1, and D2 depth semantics
    And the FRESH instruction should tell the agent to call inject_summary

  Scenario: Second compaction uses INCREMENTAL instruction when existing DAG found
    Given a session with conversation messages and an existing compaction-dag system-reminder
    And the existing DAG contains dag-node blocks with max turn_end of 82
    When execute_compaction is called
    Then the injected user message should contain the INCREMENTAL compaction instruction
    And the instruction should include the existing DAG content
    And the instruction should reference start_turn 83 for searching only new turns

  Scenario: detect_existing_dag finds DAG in messages
    Given a session messages list containing a user message with compaction-dag marker
    And the DAG content has dag-node blocks with turns 0-20 and 21-50
    When detect_existing_dag is called with those messages
    Then it should return Some with the DAG content string
    And the returned max_turn_end should be 50

  Scenario: detect_existing_dag returns None when no DAG exists
    Given a session messages list with only regular conversation messages
    When detect_existing_dag is called with those messages
    Then it should return None

  Scenario: Incremental template substitution replaces placeholders
    Given a compaction-dag exists with content containing architecture decisions
    And the parsed dag-nodes have a max turn_end of 95
    When the incremental instruction is constructed
    Then the placeholder {existing_dag_content} should be replaced with the actual DAG content
    And the placeholder {last_compacted_turn} should be replaced with 95

  Scenario: execute_compaction appends resume prompt for both modes
    Given a session with no existing DAG
    When execute_compaction is called with a last_user_message of "implement the login feature"
    Then the injected message should contain the FRESH instruction
    And the injected message should contain "implement the login feature" as the resume prompt
    Given a session with an existing DAG
    When execute_compaction is called with a last_user_message of "fix the test"
    Then the injected message should contain the INCREMENTAL instruction
    And the injected message should contain "fix the test" as the resume prompt

  Scenario: Existing DAG with no parseable dag-node blocks uses fallback turn_end
    Given a session with a compaction-dag system-reminder containing only plain text (no dag-node blocks)
    When detect_existing_dag is called
    Then it should return Some with the DAG content
    And the returned max_turn_end should be 0 as a fallback

  Scenario: FRESH instruction preserves current behavior
    Given the COMPACTION_INSTRUCTION_FRESH constant
    When its content is examined
    Then it should contain guidance for SessionSearch strategic searching
    And it should contain the dag-node XML block format with depth, turns, and label attributes
    And it should contain D0, D1, and D2 depth semantics
    And it should instruct the agent to call inject_summary

  Scenario: INCREMENTAL instruction contains promotion guidance
    Given the COMPACTION_INSTRUCTION_INCREMENTAL constant template
    When its content is examined
    Then it should instruct to PRESERVE existing D2 nodes unchanged
    And it should instruct to REVIEW existing D1 nodes
    And it should instruct to PROMOTE existing D0 nodes to D1
    And it should instruct to search ONLY for turns since last compaction
    And it should contain placeholders {existing_dag_content} and {last_compacted_turn}
    And it should instruct to call inject_summary with the updated DAG
