@done
@attachment-viewer
@viewer
@RPC-374
Feature: Wire A key on board to open attachment picker and browser
  """
  BoardView::handle_event (rust/fspec-tui/src/views/board.rs) gains a KeyCode::Char('a')|Char('A') arm that always returns EventResult::consumed() and emits Action::OpenAttachmentPicker only when store.selected_work_unit() has a non-empty attachments Vec. New Action variants in src/components/mod.rs: OpenAttachmentPicker and OpenAttachment(String). A new AttachmentPickerDialog component (src/components/attachment_picker_dialog.rs) modeled on create_session_dialog.rs / checkpoint_restore_dialog.rs: a Priority::Foreground modal listing one row per attachment (basename shown), Up/Down to move, Enter emits Action::OpenAttachment(full_path) + pops, Esc pops. dispatch_viewer.rs (reused from RPC-373) handles OpenAttachmentPicker by pushing the dialog built from the selected work unit's attachments onto the compositor, and OpenAttachment(path) by launching the browser at attachment_url(port, path). Pure attachment_url(port,&path)->String percent-encodes the path (matching TS encodeURI; spaces->%20) and App::attachment_target(&self,&path)->Option<String> gates on viewer_port. open::that runs only in the Some branch so no browser launches in tests. Picker list rows are unit-testable via a pub accessor. All files <300 lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing A or a always consumes the key on the board; it opens an attachment picker only when the selected work unit has at least one attachment
  #   2. Pressing A on a card with no attachments is a silent no-op (no picker action emitted), but the key is still consumed
  #   3. The attachment picker lists one selectable entry per attachment of the selected work unit, in order, showing the basename
  #   4. Selecting an attachment opens it in the browser at http://127.0.0.1:PORT/view/<percent-encoded attachment path> using the viewer server port; spaces and unicode are percent-encoded
  #   5. Selecting an attachment when no viewer server port is available is a safe no-op (no URL, no browser launch, no panic)
  #
  # EXAMPLES:
  #   1. With a selected card that has two attachments, pressing A emits Action::OpenAttachmentPicker and consumes the key
  #   2. With a selected card that has no attachments, pressing A emits no picker action but still consumes the key
  #   3. Pressing lowercase a behaves the same as uppercase A
  #   4. An attachment picker built from a work unit with attachments [spec/attachments/RPC-001/design.md, spec/attachments/RPC-001/a b.md] lists two entries showing design.md and a b.md
  #   5. With viewer port 53999, selecting attachment spec/attachments/RPC-001/a b.md targets http://127.0.0.1:53999/view/spec/attachments/RPC-001/a%20b.md
  #   6. With no viewer port set, selecting an attachment resolves to nothing and no browser is launched
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

  Scenario: Pressing A on a card with no attachments is a silent no-op
    Given a card is selected that has no attachments
    When I press the uppercase A key
    Then no open-attachment-picker action is emitted
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

  Scenario: Selecting an attachment opens it at the encoded viewer URL
    Given the attachment viewer server is running on a known port
    When I select the attachment "spec/attachments/RPC-001/a b.md"
    Then the target is the percent-encoded view URL for that attachment on that port

  Scenario: Selecting an attachment is a safe no-op when the viewer server is unavailable
    Given the attachment viewer server is not running
    When I select an attachment
    Then there is no target and no browser is launched
