@TOOL-018
Feature: HITL Freeform 'Other' Option for Predefined Choices
  """
  Changes are entirely in the TUI layer — InputTransition rendering and useHitlInput hook. No changes to the Rust request_user_input.rs tool definition, HitlRequest/HitlResponse types, NAPI bindings, or tool schema. The 'Other...' is a virtual UI-only option.
  useHitlInput hook needs a new state: isOtherActive (boolean). When true, the current question renders MultiLineInput instead of the option list. Escape while isOtherActive sets it back to false (returns to option list). Enter on the 'Other...' option index sets isOtherActive to true.
  InputTransition rendering: when hasOptions is true, map options to display items and append { label: 'Other...', description: '' } as the last entry. The 'Other...' entry uses dim/italic chalk styling. The selectedOption index wraps around length+1 (options count + 1 for Other).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a question has predefined options, the TUI MUST append an 'Other...' entry at the bottom of the option list that the user can select with ↑/↓ like any other option
  #   2. Selecting 'Other...' and pressing Enter MUST transition the current question from option-selection mode to freeform text input mode — reusing the same MultiLineInput component already used for freeform-only questions
  #   3. When the user submits via 'Other...', the answer shape MUST be { selected: [], other: "user typed text" } — same shape as a freeform-only question, so the LLM receives a consistent response format
  #   4. Pressing Escape while in the freeform text input (after selecting 'Other...') MUST go back to the option list for that question — NOT cancel the entire HITL flow
  #   5. The 'Other...' option is TUI-only — no schema changes to the tool definition or Rust types. The LLM still sends 2-3 options; the TUI appends the virtual 'Other...' entry at render time
  #   6. The 'Other...' entry MUST be visually distinct from LLM-provided options — rendered in dim/italic text to indicate it's a fallback, not one of the suggested choices
  #   7. Empty freeform submission (user hits Enter on blank text after selecting Other) MUST be rejected — show inline hint 'Please type a response or press Esc to go back'
  #
  # EXAMPLES:
  #   1. AI asks 'Which approach?' with options [A, B, C] — TUI renders: ○ A, ○ B, ○ C, ○ Other... (dim) — user navigates to 'Other...', presses Enter, types 'I want approach D which combines A and C', presses Enter — answer returned as { selected: [], other: 'I want approach D which combines A and C' }
  #   2. AI asks 'Which approach?' with options [A, B] — user navigates to 'Other...', presses Enter, sees freeform input — then presses Escape — returns to the option list with ○ A, ○ B, ○ Other... — selects 'A' instead and presses Enter — answer returned as { selected: ['A'] }
  #   3. User selects 'Other...' on question 1 of 2, types a response, presses Enter — advances to question 2 which also has options — user picks a predefined option on question 2 — both answers submitted together, question 1 has { selected: [], other: 'custom text' } and question 2 has { selected: ['Option B'] }
  #   4. User selects 'Other...', presses Enter, sees freeform input, presses Enter with empty text — sees inline hint 'Please type a response or press Esc to go back' — cursor stays in freeform input, not advanced
  #   5. Question has no options (freeform-only) — no 'Other...' is appended because the entire question is already freeform — behavior unchanged from current implementation
  #
  # ========================================
  Background: User Story
    As a developer using the HITL input modal
    I want to provide a custom freeform response when none of the predefined options match my intent
    So that I'm not forced to pick an incorrect option just because the AI didn't anticipate my answer

  @integration
  Scenario: Select Other and submit freeform response
    Given the AI presents a HITL question with options "A", "B", and "C"
    Then the TUI renders the options with an appended "Other..." entry in dim text
    When I navigate to "Other..." and press Enter
    Then the option list is replaced by a freeform text input
    When I type "I want approach D which combines A and C" and press Enter
    Then the answer is returned as selected [] and other "I want approach D which combines A and C"

  @integration
  Scenario: Escape from Other freeform returns to option list
    Given the AI presents a HITL question with options "A" and "B"
    When I navigate to "Other..." and press Enter
    Then the option list is replaced by a freeform text input
    When I press Escape
    Then the option list is displayed again with "A", "B", and "Other..."
    When I navigate to "A" and press Enter
    Then the answer is returned as selected ["A"]

  @integration
  Scenario: Mixed Other and predefined across multi-question flow
    Given the AI presents 2 HITL questions each with predefined options
    When I select "Other..." on question 1 and type "custom text" and press Enter
    Then the flow advances to question 2
    When I select "Option B" on question 2 and press Enter
    Then both answers are submitted with question 1 as selected [] other "custom text" and question 2 as selected ["Option B"]

  Scenario: Empty freeform submission is rejected
    Given the AI presents a HITL question with options "A" and "B"
    When I navigate to "Other..." and press Enter
    Then the option list is replaced by a freeform text input
    When I press Enter with empty text
    Then I see an inline hint "Please type a response or press Esc to go back"
    And the cursor remains in the freeform text input

  Scenario: Freeform-only question does not show Other
    Given the AI presents a HITL question without predefined options
    Then the TUI renders only a freeform text input
    And no "Other..." entry is displayed
