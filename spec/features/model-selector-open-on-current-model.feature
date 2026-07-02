@done
@RPC-341
@tui
@model-selector
Feature: Model selector opens on the current model
  """
  Dispatch order is favorable: set_current_model is called before set_providers (dispatch_model_selector.rs:28-29 then :42-43), so cursor seeding happens synchronously inside set_providers — no TS-style hasAutoExpanded latch needed
  Add rows::index_of_model(rows, current_model_id) helper (selectable guard so headers with empty model_id can't match); seed selected_index in set_providers, else keep validate-or-first-selectable fallback
  RECONCILED (re-review): this card was shipped together with RPC-342, so set_providers now collapses all sections by default and auto-expands ONLY the current model's section (this is the more TS-faithful behavior). The earlier "keep expand-all" note and the stale mod.rs:474 is_expanded('openai') reference no longer apply; the cursor seeding remains the focus of this card and the expand-only-current behavior is covered by RPC-342.
  KNOWN LIMITATION: matching is by model id only (TS parity, ModelSelectorScreen.tsx:98-109); if the same model id appears under two providers the earlier provider's copy is highlighted. No example-map case covers a duplicate-model-id collision.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the selector loads providers and a current model is set, the cursor lands on the selectable row whose model_id matches the current model
  #   2. When no current model is set (None), the cursor falls back to the first selectable row
  #   3. When the current model id matches no loaded model, the cursor falls back to the first selectable row
  #   4. A non-selectable header row (empty model_id) can never be matched as the current model
  #   5. After seeding the cursor on the current model, the scroll offset is adjusted so the seeded row is visible (RPC-340 interaction)
  #
  # EXAMPLES:
  #   1. Current model is 'claude-sonnet' (in the anthropic section, after the openai section); selector opens with cursor on the 'claude-sonnet' row, not 'gpt-4o'
  #   2. No current model set; selector opens with cursor on the first selectable row
  #   3. Current model id 'does-not-exist' matches no loaded model; cursor falls back to the first selectable row
  #   4. Current model lives in a long list below the viewport fold; selector opens with cursor on it AND the row scrolled into view
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to have the model selector open with the cursor already on my current model
    So that I can see and re-confirm my active model without hunting for it

  Scenario: Cursor lands on the current model when it is loaded
    Given my current model is "claude-sonnet"
    When the model selector loads the "openai" and "anthropic" providers
    Then the cursor is on the selectable row for "claude-sonnet"
    And the cursor is not on the first model "gpt-4o"

  Scenario: Cursor falls back to the first selectable row when no current model is set
    Given no current model is set
    When the model selector loads the providers
    Then the cursor is on the first selectable row

  Scenario: Cursor falls back to the first selectable row when the current model is not found
    Given my current model is "does-not-exist"
    When the model selector loads the providers
    Then the cursor is on the first selectable row

  Scenario: Seeded cursor on a below-the-fold model is scrolled into view
    Given my current model is in a long list below the viewport fold
    When the model selector loads the providers
    Then the cursor is on the selectable row for my current model
    And the seeded row is scrolled into view
