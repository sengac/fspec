@done
@BUG-431
@tui
@attachment-viewer
@rust
Feature: Card attachments are read from the daemon
  """
  fspec-tui app/board_card_attachments.rs resolved a card attachment against
  the workspace path in the details envelope and the attachment viewer served
  it from the client's filesystem, so with a remote daemon the path named a
  directory on another machine (missing, or a wrong local file). The daemon
  now serves attachment bytes: codelet_attachment_viewer::
  read_workspace_attachment validates the path against the workspace (lexical
  escape check, missing / directory / non-regular / oversized failures) and
  the RPC read_card_attachment (appended to the frozen wire order, MCP tool)
  calls it in the opened workspace. The TUI fetches through
  FspecBackend::read_card_attachment, writes the bytes into a per-app cache
  directory under the attachment's workspace-relative path, and serves that
  cache with the viewer.
  Tests: rust/fspec-tui/tests/bug431_card_attachments_come_from_the_daemon.rs.
  """

  Background: User Story
    As a user viewing a card's attachments while connected to a remote daemon
    I want to open each attachment from the daemon
    So that attachments open even though the files live on another machine, and never as a wrong local file

  Scenario: An attachment in a workspace that exists only on the daemon opens
    Given a card whose workspace exists only on the daemon
    When an attachment of that card is opened
    Then the viewer serves the daemon's bytes for it

  Scenario: The daemon's refusal is shown for an attachment it cannot serve
    Given a card whose workspace exists only on the daemon
    When an attachment the daemon cannot serve is opened
    Then the attachment fails with the daemon's reason and no browser opens
