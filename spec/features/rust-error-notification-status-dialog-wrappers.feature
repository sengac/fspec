@done
@rust
@ui-enhancement
@dialog
@tui
@RPC-079
Feature: Port reusable ErrorDialog/NotificationDialog/StatusDialog wrappers from TS Ink to Rust ratatui via dialog_theme::render_dialog
  """
  TS source files: src/components/ErrorDialog.tsx, src/components/NotificationDialog.tsx, src/components/StatusDialog.tsx (read-only references)
  Rust port targets: rust/fspec-tui/src/components/error_dialog.rs, notification_dialog.rs, status_dialog.rs (new). Register in rust/fspec-tui/src/components/mod.rs.
  Canonical primitive: rust/fspec-tui/src/components/dialog_theme.rs::render_dialog. Reuse Accent enum, FspecDialog struct, MARKER_SELECTED/MARKER_UNSELECTED/FOOTER_SEPARATOR constants.
  Reference pattern for new dialogs: rust/fspec-tui/src/components/disconnect_dialog.rs — shortest example of Component impl + render_dialog delegation + Callback-based dismissal.
  Auto-dismiss timing: use tokio::time::sleep + a oneshot dismissal channel, OR rely on the Action::Tick pattern already used in scrollback (count down on each tick). Follow whichever pattern is established in pause_dialog.rs / model_selector_dialog.rs for consistency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ErrorDialog renders Accent::Red, title 'Error', single message body row, footer 'Press ESC to dismiss', sticky (no auto-dismiss)
  #   2. NotificationDialog supports three severities: Success→Accent::Cyan border with green title text, Info→Accent::Cyan border with cyan title text, Warning→Accent::Yellow border with yellow title text
  #   3. NotificationDialog auto-dismisses after auto_dismiss_ms (default 2000ms, 0 disables) and shows live 'Closing in Ns... (ESC to dismiss)' countdown in the footer
  #   4. StatusDialog is a state machine: Restoring (Accent::Cyan, current item + 'idx/total' counter, no dismissal), Complete (Accent::Cyan with green title, auto-closes after 3s with countdown, ESC skips), Error (Accent::Red, error_message body, ESC dismisses)
  #   5. Every new dialog struct must delegate rendering to dialog_theme::render_dialog(area, buf, &FspecDialog{..}); hand-rendering with Block/Paragraph is rejected
  #   6. Every new dialog implements Component with Priority::Critical, exposes a const ID (e.g. ERROR_DIALOG_ID, NOTIFICATION_DIALOG_ID, STATUS_DIALOG_ID), and emits a Callback that calls compositor.remove(&id) on dismissal
  #   7. ESC key handling: ErrorDialog dismisses; NotificationDialog dismisses (cancels auto-dismiss timer); StatusDialog ESC only active in Complete and Error states, ignored during Restoring
  #   8. All three dialogs ship with insta snapshot tests on an 80x24 buffer + behaviour tests asserting Component::priority(), id(), handle_event(ESC), and state transitions
  #   9. Every existing inline-FspecDialog consumer in rust/fspec-tui/ that displays an error / notification / progress modal is refactored to use the new wrappers; no inline FspecDialog struct literals remain for these three semantics after this work unit
  #
  # EXAMPLES:
  #   1. ErrorDialog::new('Disk full') rendered into an 80x24 TestBackend produces a centered red rounded border, bold red 'Error' title, red 'Disk full' body row, dim centered 'Press ESC to dismiss' footer
  #   2. NotificationDialog::success('Saved') with default auto_dismiss_ms=2000 shows green title 'Success', cyan border, 'Closing in 2s... (ESC to dismiss)' footer; after 2s the dialog emits a dismissal Callback
  #   3. NotificationDialog::warning('Slow connection') with auto_dismiss_ms=0 shows yellow title 'Warning', yellow border, static 'Press ESC to dismiss' footer (no countdown)
  #   4. StatusDialog in Restoring state with current='file3.txt' idx=3 total=10 shows cyan border, cyan 'Restoring Files' title, 'file3.txt' body, '(3/10)' counter; ESC is ignored (no Callback emitted)
  #   5. StatusDialog transitions Restoring→Complete: cyan border with green title 'Restore Complete!', auto-closes after 3s with 'Closing in Ns...' countdown, ESC skips the wait
  #   6. StatusDialog transitions Restoring→Error: red border, bold red 'Error' title, red error_message body, ESC dismisses the dialog
  #   7. Every new dialog test fixture that grep-asserts the rendered buffer NEVER finds inline FspecDialog struct usage in non-test code (verified by clippy lint or repo grep)
  #
  # ========================================
  Background: User Story
    As a Rust ratatui developer adding modal feedback to a view
    I want to drop in a single struct call for error / notification / progress modals
    So that the visual contract stays byte-equal to the TS Ink reference and a future dialog-theme change lands in exactly one place

  Scenario: ErrorDialog renders red bordered modal with sticky ESC-only dismissal
    Given an ErrorDialog constructed with message "Disk full"
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the buffer contains a centered rounded border drawn in Color::Red
    Then the title row reads "Error" in bold Color::Red
    Then the body contains a row whose visible text equals "Disk full" in Color::Red
    Then the footer reads "Press ESC to dismiss" in dim style centered horizontally
    Then no auto-dismiss Callback fires for at least 5 seconds
    When the dialog receives a KeyCode::Esc event
    Then the dialog emits a Callback that calls compositor.remove(ERROR_DIALOG_ID)

  Scenario: NotificationDialog success severity shows cyan border with green title and 2s countdown
    Given a NotificationDialog constructed with message "Saved" and severity Success
    Given auto_dismiss_ms is left at its default of 2000
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the buffer contains a centered rounded border drawn in Color::Cyan
    Then the title row reads "Success" in bold Color::Green
    Then the body contains a row whose visible text equals "Saved"
    Then the footer reads "Closing in 2s... (ESC to dismiss)" in dim style centered horizontally
    When 1 second of simulated time elapses
    Then the footer reads "Closing in 1s... (ESC to dismiss)"
    When a further 1 second of simulated time elapses
    Then the dialog emits a Callback that calls compositor.remove(NOTIFICATION_DIALOG_ID)

  Scenario: NotificationDialog warning severity with auto_dismiss_ms=0 is sticky with yellow border
    Given a NotificationDialog constructed with message "Slow connection" and severity Warning
    Given auto_dismiss_ms is set to 0
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the buffer contains a centered rounded border drawn in Color::Yellow
    Then the title row reads "Warning" in bold Color::Yellow
    Then the body contains a row whose visible text equals "Slow connection"
    Then the footer reads "Press ESC to dismiss" in dim style centered horizontally
    Then no auto-dismiss Callback fires for at least 5 seconds
    When the dialog receives a KeyCode::Esc event
    Then the dialog emits a Callback that calls compositor.remove(NOTIFICATION_DIALOG_ID)

  Scenario: StatusDialog in Restoring state shows progress counter and ignores ESC
    Given a StatusDialog constructed with operation_type "Restoring Files"
    When the dialog enters Restoring state with current="file3.txt", idx=3, total=10
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the buffer contains a centered rounded border drawn in Color::Cyan
    Then the title row reads "Restoring Files" in bold Color::Cyan
    Then the body contains a row whose visible text equals "file3.txt"
    Then the body contains a row whose visible text equals "(3/10)"
    When the dialog receives a KeyCode::Esc event
    Then the dialog returns EventResult::ignored() and emits NO Callback

  Scenario: StatusDialog transitions Restoring to Complete with green title and 3s auto-close
    Given a StatusDialog currently in Restoring state with operation_type "Restoring Files"
    When the dialog transitions to Complete state
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the buffer contains a centered rounded border drawn in Color::Cyan
    Then the title row reads "Restore Complete!" in bold Color::Green
    Then the footer reads "Closing in 3s... (ESC to dismiss)" in dim style centered horizontally
    When 3 seconds of simulated time elapse
    Then the dialog emits a Callback that calls compositor.remove(STATUS_DIALOG_ID)
    When a fresh StatusDialog enters Complete state and receives a KeyCode::Esc event before the countdown finishes
    Then the dialog emits a Callback that calls compositor.remove(STATUS_DIALOG_ID) immediately

  Scenario: StatusDialog transitions Restoring to Error with red border and ESC dismissal
    Given a StatusDialog currently in Restoring state
    When the dialog transitions to Error state with error_message "Permission denied: /etc/passwd"
    When the dialog is rendered into an 80x24 TestBackend buffer
    Then the buffer contains a centered rounded border drawn in Color::Red
    Then the title row reads "Error" in bold Color::Red
    Then the body contains a row whose visible text equals "Permission denied: /etc/passwd" in Color::Red
    Then no auto-dismiss Callback fires
    When the dialog receives a KeyCode::Esc event
    Then the dialog emits a Callback that calls compositor.remove(STATUS_DIALOG_ID)

  Scenario: No raw FspecDialog struct literals remain in non-test code after the wrappers ship
    Given the rust/fspec-tui crate after RPC-079 implementation completes
    When a grep search for "FspecDialog {" is run across rust/fspec-tui/src/ excluding the components/ directory and excluding all #[cfg(test)] blocks
    Then zero matches are returned
    When the same search is run inside rust/fspec-tui/src/components/
    Then the only matches occur inside render() methods of files that delegate to dialog_theme::render_dialog

  Scenario: ErrorDialog is shown when an LLM provider error chunk arrives
    Given an App with an active session and no error dialog currently on the compositor
    When the App dispatches Action::ChunkReceived for that session with StreamChunk::Error{error: "provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests"}
    Then the Compositor contains a layer with id ERROR_DIALOG_ID at Priority::Critical
    Then the scrollback for that session still contains the 'API Error: provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests' line per RPC-078

  Scenario: End-to-end App.render paints ErrorDialog modal on top of AgentView when a provider Error chunk arrives
    Given an App with an active session s-1 routed to the AgentView and no error dialog currently on the compositor
    When the App dispatches Action::ChunkReceived(s-1, StreamChunk::Error{error: "provider error: [claude] API error: Rig completion failed: HttpError: Invalid status code 429 Too Many Requests"}) and then App::render is called into an 80x24 TestBackend buffer
    Then the rendered buffer contains a centered rounded red border drawn ON TOP of the AgentView scrollback (i.e. the ErrorDialog modal is painted last and covers the centre of the 80x24 buffer), with the bold red 'Error' title text visible inside the border and the scrollback 'API Error:' text still present in the rows the modal does not cover
