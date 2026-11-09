@BUG-582
@done
@attachment-viewer
@viewer
@RPC-374
@BUG-469
Feature: Wire A key on board to open attachment picker and browser
  """
  BoardView::handle_event (rust/fspec-tui/src/views/board.rs) gains a KeyCode::Char('a')|Char('A') arm that always returns EventResult::consumed() and emits Action::OpenAttachmentPicker only when store.selected_work_unit() has a non-empty attachments Vec. New Action variants in src/components/mod.rs: OpenAttachmentPicker and OpenAttachment(String). A new AttachmentPickerDialog component (src/components/attachment_picker_dialog.rs) modeled on create_session_dialog.rs / checkpoint_restore_dialog.rs: a Priority::Foreground modal listing one row per attachment (basename shown), Up/Down to move, Enter emits Action::OpenAttachment(full_path) + pops, Esc pops. dispatch_viewer.rs (reused from RPC-373) handles OpenAttachmentPicker by pushing the dialog built from the selected work unit's attachments onto the compositor, and OpenAttachment(path) by reading the attachment from the daemon for the selected workspace and opening it in the viewer (BUG-469, the TUI-120 App::spawn_workspace_file_open path). Pure attachment_url(port,&path)->String percent-encodes the path (matching TS encodeURI; spaces->%20). Picker list rows are unit-testable via a pub accessor. All files <300 lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing A or a always consumes the key on the board; it opens an attachment picker only when the selected work unit has at least one attachment
  #   2. TUI-129 (replaces the silent no-op): pressing A on a card with no attachments emits the picker action and the App says the card has no attachments; the key is consumed
  #   3. The attachment picker lists one selectable entry per attachment of the selected work unit, in order, showing the basename
  #   4. BUG-469: selecting an attachment reads it from the daemon for the board's selected workspace and opens it in the viewer at a percent-encoded URL (the TUI-120 fetch/cache/serve path)
  #   5. BUG-469: before the workspace list loads, or when the read fails, the reason is shown
  #
  # EXAMPLES:
  #   1. With a selected card that has two attachments, pressing A emits Action::OpenAttachmentPicker and consumes the key
  #   2. With a selected card that has no attachments, pressing A emits the picker action (the App shows 'No attachments on RPC-001') and consumes the key
  #   3. Pressing lowercase a behaves the same as uppercase A
  #   4. An attachment picker built from a work unit with attachments [spec/attachments/RPC-001/design.md, spec/attachments/RPC-001/a b.md] lists two entries showing design.md and a b.md
  #   5. With workspace beta selected, picking spec/attachments/BETA-001/a b.md reads it from beta on the daemon and opens .../view/spec/attachments/BETA-001/a%20b.md
  #   6. Before the workspace list loads, picking an attachment reads nothing and shows why
  #
  # ========================================
  Background: User Story
    As a fspec board user
    I want to press the A key on a card to pick one of its attachments and open it in my browser
    So that I can view a work unit's diagrams and documents without hunting for the files manually

  Scenario: Pressing A on a card with attachments opens the picker
    Given a card is selected that has two attachments
    When I press the uppercase A key
    Then the open-attachment-picker action is emitted
    And the key event is consumed

  Scenario: Pressing A on a card with no attachments leaves it to the App to say so
    Given a card is selected that has no attachments
    When I press the uppercase A key
    Then the open-attachment-picker action is emitted, and the App says the card has no attachments (TUI-129)
    And the key event is consumed

  Scenario: Pressing lowercase a behaves the same as uppercase A
    Given a card is selected that has two attachments
    When I press the lowercase a key
    Then the open-attachment-picker action is emitted
    And the key event is consumed

  Scenario: The picker lists the selected work unit's attachments
    Given a work unit with attachments "spec/attachments/RPC-001/design.md" and "spec/attachments/RPC-001/a b.md"
    When the attachment picker is built for that work unit
    Then the picker lists two entries
    And the entries show the basenames "design.md" and "a b.md"

  Scenario: Selecting an attachment opens it from the selected workspace on the daemon
    Given the board shows workspace "beta"
    When I select the attachment "spec/attachments/BETA-001/a b.md"
    Then it is read from "beta" on the daemon
    And the viewer opens its percent-encoded view URL

  Scenario: Selecting an attachment before the workspace list loads shows why nothing opened
    Given the workspace list has not loaded
    When I select an attachment
    Then nothing is read from the daemon and no browser is launched
    And an error explains that no workspace is selected
