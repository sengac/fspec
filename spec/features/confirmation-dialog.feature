@done
@modal
@react
@ink
@ui-enhancement
@tui
@critical
@TUI-011
Feature: Reusable confirmation dialog component for destructive TUI actions
  """
  Maps riskLevel prop (low/medium/high) to borderColor prop (green/yellow/red) before passing to Dialog
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. No existing confirmation dialog component exists in the TUI - each feature would need to implement their own
  #   2. Confirmation dialog must be non-blocking and integrate with Ink component lifecycle
  #   3. Dialog must support different confirmation modes: simple Y/N, typed confirmation phrase, single key press
  #   4. Dialog must clearly display the action being confirmed and its consequences
  #   5. Dialog must support cancellation with ESC key
  #   6. Modal overlay in the center of the screen
  #   7. Yes, support custom styling/colors for different risk levels (low=green, medium=yellow, high=red) as an optional prop
  #   8. Component API: Props should include: message (required), onConfirm callback (required), onCancel callback (required), confirmMode ('yesno' | 'typed' | 'keypress', default 'yesno'), typedPhrase (required if confirmMode='typed'), riskLevel ('low' | 'medium' | 'high', optional), description (optional additional details)
  #   9. Default dialog appearance is a white/neutral modal without risk level styling
  #   10. ConfirmationDialog extends a base Dialog component that provides modal overlay infrastructure
  #   11. Dialog component (base): Handles ONLY modal overlay infrastructure - centering, borders, ESC key, layout. No business logic.
  #   12. ConfirmationDialog component: Handles ONLY confirmation-specific logic - modes (yesno/typed/keypress), validation, onConfirm/onCancel. Uses Dialog for rendering.
  #   13. ConfirmationDialog can override or extend Dialog's key handlers - Dialog provides base ESC handling, ConfirmationDialog adds mode-specific key handling
  #   14. Dialog component's useInput handler must not interfere with parent TUI component key handlers - uses isActive prop pattern to control when it captures input
  #
  # EXAMPLES:
  #   1. User presses Delete on checkpoint, dialog shows 'Delete checkpoint AUTH-001-auto-testing? [y/N]', user presses Y, checkpoint deleted
  #   2. User presses Shift+D to delete all checkpoints, dialog shows 'Type DELETE ALL to confirm:', user types DELETE ALL, all checkpoints deleted
  #   3. User presses R to restore file, dialog shows 'Restore file will overwrite uncommitted changes. Continue? [y/N]', user presses ESC, action cancelled
  #   4. Developer shows dialog for non-destructive confirmation (e.g., 'Save changes?'), no riskLevel prop provided, dialog displays with standard white/neutral styling
  #   5. Developer creates custom InfoDialog extending Dialog with just a message and OK button - Dialog provides modal overlay, InfoDialog provides OK logic
  #   6. Dialog component renders centered modal with red border when borderColor='red' prop provided, handles ESC to call onClose - no confirmation logic
  #   7. User is in BoardDisplay with active keyboard navigation, opens confirmation dialog - dialog captures ALL input, board navigation suspended until dialog closes
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the dialog be a modal overlay (center of screen) or inline at the bottom like a prompt bar?
  #   A: true
  #
  #   Q: Should the dialog support custom styling/colors for different risk levels (low=green, medium=yellow, high=red)?
  #   A: true
  #
  #   Q: What should the component API look like? Props: message, confirmMode, onConfirm callback, onCancel callback, riskLevel?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a TUI developer
    I want to use a reusable confirmation dialog for destructive actions
    So that I can implement consistent, safe confirmation flows without duplicating code

  Scenario: Y/N mode calls onConfirm when Y pressed
    Given a ConfirmationDialog with confirmMode='yesno' and message 'Delete checkpoint?'
    When the user presses the Y key
    Then the onConfirm callback should be called

  Scenario: Typed mode requires exact phrase match
    Given a ConfirmationDialog with confirmMode='typed' and typedPhrase='DELETE ALL'
    When the user types 'DELETE ALL' and presses Enter
    Then the onConfirm callback should be called

  Scenario: Risk level maps to Dialog border color
    Given a ConfirmationDialog with riskLevel='high'
    When the dialog renders
    Then the Dialog component should receive borderColor='red'

  Scenario: No risk level defaults to neutral styling
    Given a ConfirmationDialog with no riskLevel prop
    When the dialog renders
    Then the Dialog component should receive borderColor=undefined
