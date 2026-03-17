@CMPCT-021
Feature: File ID Propagation Through DAG

  """
  Engine-side: build_dag_files_block in inject_summary_handler.rs, auto-append to DAG in apply_pending_dag after agent content stored
  Agent-side: Update COMPACTION_INSTRUCTION_INCREMENTAL in interactive_helpers.rs to mention preserving/updating <dag-files> section
  FileModification annotations from codelet_core::compaction::model (StructuralAnnotation::FileModification with FileOp enum), persisted in StoredMessage.metadata["annotations"]
  Use BTreeMap<String, FileOp> for deterministic ordering of file paths in <dag-files> block. Parse format: '- path/to/file (Created|Modified|Deleted)'
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After inject_summary, the engine must scan FileModification annotations from compacted turns and append a <dag-files> block to the stored DAG if the agent omitted one
  #   2. If the agent already included a <dag-files> block in their DAG, the engine must NOT duplicate it — use the agent's version
  #   3. During incremental compaction, the previous DAG's <dag-files> block must be included in the instruction so the agent carries file awareness forward
  #   4. The dag-files block must merge existing files with new FileModification annotations, with newer operations overriding older ones
  #   5. If no FileModification annotations exist and there is no existing dag-files block, no <dag-files> block should be appended (no empty blocks)
  #   6. The incremental compaction instruction must explicitly tell the agent to PRESERVE and UPDATE the <dag-files> section
  #
  # EXAMPLES:
  #   1. Agent omits dag-files block, engine has 3 FileModification annotations (Created, Modified, Deleted) → engine appends <dag-files> block with all 3 entries
  #   2. Agent includes their own <dag-files> block → engine does NOT append a duplicate, agent's version is used as-is
  #   3. Second compaction: existing dag-files has [a.rs(Created), b.rs(Modified)], new annotations have [a.rs(Modified), c.rs(Created)] → merged result has [a.rs(Modified), b.rs(Modified), c.rs(Created)]
  #   4. No FileModification annotations and no existing dag-files → no <dag-files> block appended
  #   5. Incremental compaction instruction includes existing DAG with dag-files section → agent can see previously modified files and update them
  #   6. Parse existing <dag-files> block from DAG content → extracts list of (path, operation) tuples correctly
  #
  # ========================================

  Background: User Story
    As a AI coding agent
    I want to have file references (paths I modified) propagate through DAG compaction cycles
    So that I retain awareness of which files I previously worked with and don't waste turns re-discovering them

  Scenario: Engine appends dag-files block when agent omits it
    Given a session with FileModification annotations for 3 files
      | path           | operation |
      | src/auth.rs    | Created   |
      | src/db.rs      | Modified  |
      | src/old.rs     | Deleted   |
    And the agent's DAG content does not contain a dag-files block
    When apply_pending_dag processes the agent's DAG content
    Then the stored DAG should have a dag-files block appended
    And the dag-files block should contain 3 file entries
    And the entries should be sorted alphabetically by path

  Scenario: Agent-provided dag-files block is preserved without duplication
    Given a session with FileModification annotations for "src/auth.rs" as Created
    And the agent's DAG content already contains a dag-files block
    When apply_pending_dag processes the agent's DAG content
    Then the stored DAG should contain exactly one dag-files block
    And it should be the agent's original dag-files block unchanged

  Scenario: Merge existing dag-files with new annotations across compactions
    Given an existing dag-files block with entries
      | path      | operation |
      | a.rs      | Created   |
      | b.rs      | Modified  |
    And new FileModification annotations
      | path      | operation |
      | a.rs      | Modified  |
      | c.rs      | Created   |
    When build_dag_files_block merges existing entries with new annotations
    Then the merged result should contain 3 file entries
    And "a.rs" should have operation "Modified" overriding "Created"
    And "b.rs" should have operation "Modified" carried forward
    And "c.rs" should have operation "Created" as a new entry

  Scenario: No dag-files block when no file modifications exist
    Given a session with no FileModification annotations
    And no existing dag-files block
    When build_dag_files_block is called
    Then it should return None
    And no dag-files block should be appended to the DAG

  Scenario: Incremental compaction instruction includes dag-files preservation guidance
    Given the COMPACTION_INSTRUCTION_INCREMENTAL constant
    When the instruction text is examined
    Then it should contain guidance to preserve the dag-files section
    And it should mention updating dag-files with new file modifications

  Scenario: Parse dag-files block from existing DAG content
    Given a DAG content string containing a dag-files block with entries
      | path             | operation |
      | src/handler.rs   | Created   |
      | src/types.rs     | Modified  |
    When the dag-files block is parsed
    Then it should extract 2 file entries
    And "src/handler.rs" should be parsed as Created
    And "src/types.rs" should be parsed as Modified

  Scenario: Fresh compaction instruction mentions dag-files in Active Files section
    Given the COMPACTION_INSTRUCTION_FRESH constant
    When the instruction text is examined
    Then it should contain guidance for including an Active Files section with dag-files

  Scenario: dag-files block format uses deterministic path ordering
    Given FileModification annotations in non-alphabetical order
      | path      | operation |
      | z.rs      | Created   |
      | a.rs      | Modified  |
      | m.rs      | Created   |
    When build_dag_files_block generates the block
    Then the entries should appear in alphabetical order: a.rs, m.rs, z.rs
