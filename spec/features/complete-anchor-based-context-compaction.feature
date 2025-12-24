@done
@codelet
@compaction
@context-management
@CTX-001
Feature: Complete Anchor-Based Context Compaction

  """
  Key architectural decisions:
  - Uses anchor detection patterns to identify meaningful conversation milestones
  - Implements PreservationContext struct for dynamic context extraction
  - Creates synthetic UserCheckpoint anchors when no natural anchors are detected
  - Supports multiple anchor types: ErrorResolution (0.9), TaskCompletion (0.8), FeatureMilestone (0.75), UserCheckpoint (0.7)

  Dependencies and integrations:
  - codelet/core/src/compaction/model.rs - PreservationContext, BuildStatus, ConversationTurn
  - codelet/core/src/compaction/anchor.rs - Anchor detection logic
  - codelet/core/src/compaction/compactor.rs - Summary generation using PreservationContext

  Critical implementation requirements:
  - PreservationContext MUST contain: active_files, current_goals, error_states, build_status, last_user_intent
  - BuildStatus MUST have variants: Passing, Failing, Unknown
  - Summary generation MUST use PreservationContext.format_for_summary() - NO hardcoded text
  - Synthetic anchor MUST be created when no natural anchors found
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (12 Rules, 10 Examples)
  # ========================================
  #
  # BUSINESS RULES:
  #   [0] Anchors are only created when detection confidence >= 0.9 (90%)
  #   [1] ErrorResolution anchors have weight 0.9 (highest), TaskCompletion 0.8, FeatureMilestone 0.75, UserCheckpoint 0.7
  #   [2] Always preserve last 2-3 conversation turns regardless of anchors
  #   [3] When no natural anchors are detected, create a synthetic UserCheckpoint at conversation end
  #   [4] Warn when compression ratio < 60%
  #   [5] Extract active files, goals, error states, and build status from conversation into PreservationContext
  #   [6] PreservationContext struct MUST contain: active_files, current_goals, error_states, build_status, last_user_intent
  #   [7] BuildStatus enum MUST have variants: Passing, Failing, Unknown
  #   [8] PreservationContext::extract_from_turns() MUST extract active files from Edit/Write/Read tool calls
  #   [9] Summary generation MUST use dynamic PreservationContext.format_for_summary() - NO hardcoded placeholder text
  #   [10] detect_bash_milestone() MUST create TaskCompletion anchor when bash succeeds with milestone keywords
  #   [11] detect_successful_search() MUST create UserCheckpoint anchor when web_search succeeds with synthesis
  #
  # EXAMPLES:
  #   [0] Coding conversation: Edit auth.rs + tests pass → TaskCompletion anchor at turn 3
  #   [1] Web search conversation: no Edit/Write tools → synthetic UserCheckpoint at last turn
  #   [2] Mixed conversation: npm install success → bash milestone anchor
  #   [3] PreservationContext extraction: active_files=['auth.rs','README.md'], build_status=Passing
  #   [4] Synthetic anchor fallback: 10 turns with only web_search → UserCheckpoint at turn 9
  #   [5] PreservationContext with 5 turns: active_files=['auth.rs','login.ts','config.json'], build_status=Passing
  #   [6] Dynamic summary: contains 'Active files: auth.rs' NOT '[from conversation]'
  #   [7] Goal extraction: 'Please fix the auth bug' → current_goals=['Fix the auth bug']
  #   [8] Build status detection: 'All 15 tests passed' → build_status=Passing
  #   [9] Error state extraction: 'error: cannot find module' → error_states contains error message
  #
  # ========================================

  Background: User Story
    As a AI agent using codelet
    I want to have meaningful context preserved after compaction with dynamic extraction of active files, goals, error states, and build status into summaries
    So that I can continue working effectively after context compaction without losing critical information about what files I was editing, what goals I was pursuing, and what the build state was

  # ========================================
  # ANCHOR DETECTION SCENARIOS
  # ========================================

  Scenario: Detect TaskCompletion anchor after successful test run
    Given a conversation with 6 turns about fixing a bug in auth.rs
    And turn 3 contains an Edit tool call to auth.rs followed by a test run that passes
    And there is no previous error state
    When I run anchor detection on turn 3
    Then a TaskCompletion anchor should be detected with confidence >= 0.9
    And the anchor should have weight 0.8
    And the anchor turn_index should be 3

  Scenario: Detect ErrorResolution anchor after fixing build error
    Given a conversation where turn 2 had a build error
    And turn 3 contains an Edit tool call followed by a test run that passes
    And previous_error is set to true for turn 3
    When I run anchor detection on turn 3
    Then an ErrorResolution anchor should be detected with confidence >= 0.9
    And the anchor should have weight 0.9

  Scenario: Detect bash milestone anchor for successful npm install
    Given a conversation with a turn containing a Bash tool call
    And the bash command output contains "packages are successfully installed"
    And the tool result success is true
    When I run anchor detection on that turn
    Then a TaskCompletion anchor should be detected with weight 0.8
    And the anchor description should contain "Bash" or "milestone"

  Scenario: Detect web search anchor with synthesis
    Given a conversation with a turn containing a WebSearch tool call
    And the search result has more than 100 characters
    And the assistant response contains "Based on the search results"
    When I run anchor detection on that turn
    Then a UserCheckpoint anchor should be detected with weight 0.7
    And the anchor description should mention web search

  Scenario: Reject anchor with confidence below threshold
    Given a conversation turn with an Edit tool call
    And the tool result does not indicate a clear test pass or fail
    When I run anchor detection with confidence threshold 0.9
    Then no anchor should be detected for that turn

  # ========================================
  # SYNTHETIC ANCHOR SCENARIOS
  # ========================================

  Scenario: Create synthetic UserCheckpoint when no natural anchors found
    Given a conversation with 10 turns
    And all turns use only Read and WebSearch tools with no Edit or Write calls
    And no natural anchors are detected in any turn
    When I run compaction on the conversation
    Then the compaction result should contain a synthetic anchor
    And the synthetic anchor should be a UserCheckpoint type at the last turn
    And the synthetic anchor should have weight 0.7
    And the synthetic anchor description should indicate it is synthetic

  # ========================================
  # PRESERVATIONCONTEXT SCENARIOS
  # ========================================

  Scenario: Extract active files from Edit Write and Read tool calls
    Given a conversation with 5 turns
    And turn 1 has an Edit tool call to "src/auth.rs"
    And turn 2 has a Write tool call to "src/login.ts"
    And turn 3 has a Read tool call to "config.json"
    When I extract PreservationContext from the turns
    Then active_files should contain "auth.rs"
    And active_files should contain "login.ts"
    And active_files should contain "config.json"

  Scenario: Extract current goals from user messages
    Given a conversation with 3 turns
    And turn 1 user message is "Please fix the auth bug"
    And turn 2 user message is "Help me implement OAuth"
    When I extract PreservationContext from the turns
    Then current_goals should contain a goal about "fix" and "auth"
    And current_goals should contain a goal about "implement" and "OAuth"

  Scenario: Detect build status as Passing from test output
    Given a conversation with a turn containing test results
    And the tool result output contains "All 15 tests passed"
    When I extract PreservationContext from the turns
    Then build_status should be Passing

  Scenario: Detect build status as Failing from test output
    Given a conversation with a turn containing test results
    And the tool result output contains "FAILED: 3 tests failed"
    When I extract PreservationContext from the turns
    Then build_status should be Failing

  Scenario: Extract error states from failed tool results
    Given a conversation with a turn containing a failed Bash command
    And the tool result success is false
    And the tool result output contains "error: cannot find module xyz"
    When I extract PreservationContext from the turns
    Then error_states should contain "cannot find module xyz"

  Scenario: Extract last user intent from most recent turn
    Given a conversation with 5 turns
    And the last turn user message is "Now deploy this to production"
    When I extract PreservationContext from the turns
    Then last_user_intent should contain "deploy" and "production"

  # ========================================
  # DYNAMIC SUMMARY SCENARIOS
  # ========================================

  Scenario: Generate summary with dynamic PreservationContext not hardcoded
    Given a conversation with 8 turns that gets compacted
    And the PreservationContext has active_files containing "auth.rs" and "login.ts"
    And the PreservationContext has current_goals containing "Fix authentication bug"
    And the PreservationContext has build_status as Passing
    When I generate a summary using the compactor
    Then the summary should contain "auth.rs"
    And the summary should contain "login.ts"
    And the summary should contain "Fix authentication bug" or similar goal text
    And the summary should contain "passing" for build status
    And the summary should NOT contain "[from conversation]"
    And the summary should NOT contain "Continue development"
    And the summary should NOT contain "Build: unknown"

  Scenario: PreservationContext format_for_summary produces correct output
    Given a PreservationContext with active_files ["auth.rs", "login.ts"]
    And current_goals ["Fix auth bug", "Add OAuth"]
    And build_status Passing
    When I call format_for_summary on the PreservationContext
    Then the output should contain "Active files: auth.rs, login.ts"
    And the output should contain "Goals: Fix auth bug; Add OAuth"
    And the output should contain "Build: passing"

  # ========================================
  # TURN SELECTION SCENARIOS
  # ========================================

  Scenario: Always preserve last 3 conversation turns
    Given a conversation with 10 turns
    And an anchor exists at turn 5
    When I run turn selection on the conversation
    Then turns 7, 8, and 9 should be in the kept_turns list
    And turns 0 through 4 should be in the summarized_turns list

  Scenario: Warn when compression ratio is below 60 percent
    Given a conversation with 4 turns totaling 400 tokens
    When I run compaction with target 300 tokens
    And the compression ratio is below 60 percent
    Then the result warnings should contain a message about low compression
    And the warning should suggest starting a fresh conversation
