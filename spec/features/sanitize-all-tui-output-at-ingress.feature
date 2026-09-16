@done
@TUI-111
@tui
@ui-enhancement
Feature: Refactor: centralize terminal output sanitization at ingress for ALL TUI views
  """
  Sanitizer module moves from store/agent_view/sanitize.rs to a new crate-level terminal::sanitize module (fspec-tui/src/terminal/sanitize.rs + moved test files), re-exported via lib.rs as `pub use terminal::sanitize::sanitize_for_terminal` so existing in-crate importers (views/diff_common, views/checkpoints) keep working; existing 5 call sites migrate to the new path. Function signature and behaviour (ANSI regex, tab→2 spaces, CR drop, C0 control-char drop, newline keep) are UNCHANGED.
  terminal.rs becomes terminal/mod.rs (file move, no content change) + terminal/sanitize.rs (moved from store/agent_view/sanitize.rs) + moved test files terminal/sanitize_tests.rs, terminal/sanitize_tests_streaming.rs. store/agent_view/sanitize.rs and its two test files are deleted; the #[path]-style test module includes in sanitize.rs migrate unchanged.
  Prompt/chrome store setters sanitize these exact fields on write: set_hitl_prompt → HitlRequest.questions[].{header,question,options[].label,options[].description}; set_pause_state → PauseState.{prompt, tool_call_id}; set_exec_stdin → ExecStdinRequest.command; set_role → the role string. Sanitize before storing (HitlPromptState::new receives the sanitized request; PauseState/ExecStdinRequest/role are sanitized in the setter body).
  Mode-view + dialog ingress field lists: BoardStore::replace_work_units sanitizes WorkUnitInfo.{id,title,description,epic,attachments[]} (work_type/status are closed vocabularies — skipped); a shared fn sanitize_work_unit(&mut WorkUnitInfo) is used by both replace_work_units and AgentViewStore::sync_work_unit_contexts so the SessionHeader WU chip stays clean. ChangedFiles: ChangedFile.{path,change_type} at set_files + diff lines at set_diff_lines. Checkpoints: CheckpointInfo.{work_unit_id,name} at set_checkpoints + file rows + diff lines. ResumeSessionView::set_sessions → SessionInfo.{name,project,worktree_path,role}. SearchHistoryView rows → HistoryMatch.text. BlocklistView::set_rules → BlocklistRuleInfo.{id,pattern,reason,guidance,source} (action is closed vocabulary — skipped). Model selector: ProviderInfo/ModelInfo display_name fields via set_providers + the form's display_name on commit.
  Test strategy: (1) moved unit tests keep the TUI-100 golden behaviour; (2) proptest property on sanitize_for_terminal: output contains no \x1b and no char in the forbidden control set for arbitrary input, plus idempotence sanitize(sanitize(x)) == sanitize(x); (3) per-sink ingress unit tests (store-level: feed hostile WorkUnitInfo/HitlRequest/PauseState/ExecStdinRequest/BlocklistRuleInfo/SessionInfo/ChangedFile/CheckpointInfo through the setters and assert stored text is clean); (4) integration tests in fspec-tui/tests/ using the mock-backend harness: drive hostile StreamChunks + WorkUnitsLoaded + dialog actions through the App and assert the rendered Buffer contains no control/ANSI bytes for board card, details strip, scrollback, error dialog, loading dialog label, HITL/pause prompt, role banner, checkpoint label, changed-files rows. Existing golden-string parity tests (chunkprocessor-parity, chunk_rendering_parity, scrollback_pty) are the regression net for the record_chunk move.
  record_chunk ingress design: each visible-text arm sanitizes its payload at the top of the arm, BEFORE the chunk-processor helpers see it — Text/Thinking/UserInput/Error/UserNotification/IncomingMessage sanitize the payload string; ToolCall sanitizes the tool name + the extracted args display string (extract_tool_args_display output is sanitized when building the header in handle_tool_call); ToolResult/ToolProgress sanitize at the record_chunk arm entry and the now-redundant calls in chunk_tool_result.rs are deleted (the maybe_mark call stays AFTER sanitization, preserving rule 9). The diff path is separate: pending_tool_diff::produce_diff_strings sanitizes each source line (Edit old/new strings, Write content, and context file lines in diff_context) BEFORE diff_format encodes rows via to_line — the \u{1} elision sentinel is only introduced by the codec and must survive to parse_line. IncomingMessage sanitizes the role + body segments BEFORE the [W] prefix formatting.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The canonical terminal sanitizer lives in ONE crate-level module (terminal::sanitize) re-exported from the crate root; no view, store, or component re-implements ANSI/control-character stripping
  #   2. Sanitization happens at DATA INGRESS (store setters, the stream-chunk recorder, action handlers, dialog constructors) so that every view paints data that was cleaned exactly once; views never sanitize
  #   3. Every StreamChunk variant that produces visible scrollback text is sanitized at SessionContext::record_chunk before it is stored: Text, Thinking, UserInput, Error, UserNotification, IncomingMessage, ToolCall (name+args), ToolResult, ToolProgress
  #   4. BoardStore::replace_work_units sanitizes WorkUnitInfo fields on ingress (id, title, description, epic, attachment paths) so BoardView cards, details strip, search dialog and mux board pane all render clean text
  #   5. Store setters that write user-visible prompt/chrome text sanitize on write: set_hitl_prompt (headers, questions, options, descriptions), set_pause_state (prompt, details/tool_call_id), set_exec_stdin (prompt, details), set_role (role text)
  #   6. Dialog constructors sanitize caller-supplied text in new(): ErrorDialog (message), NotificationDialog (message), StatusDialog (message), LoadingDialog (title + label — catches cascade stage labels that embed file paths / checkpoint names)
  #   7. Mode-view state setters sanitize incoming rows on ingress: ChangedFilesView set_files/set_diff_lines, CheckpointsView set_checkpoints/set_files/set_diff_lines, ResumeSessionView set_sessions, SearchHistoryView rows, BlocklistView set_rules (id, pattern, reason, guidance, source), model-selector rows
  #   8. Edit/Write tool-diff SOURCE lines (tool input strings + context file lines read from disk) are sanitized BEFORE the diff codec encodes them with the \u{1} elision sentinel — the encoded canonical lines are never run through the sanitizer post-encode
  #   9. The LLM receives RAW unsanitized tool output (TUI-100 rule 6 is preserved): sanitization happens only inside fspec-tui; rust/tools and the session engine are untouched
  #   10. Stderr marker ordering is preserved: ToolProgress/ToolResult text is sanitized FIRST, then the STDERR_MARKER (⚠stderr⚠) is applied by maybe_mark; the marker is still stripped at render time by strip_marker
  #
  # EXAMPLES:
  #   1. A work unit whose title contains ANSI color codes and a tab (e.g. from an LLM-authored fspec create-story) displays as plain text in the BoardView card label, the details strip title row, and the work-unit search dialog — no escape glyphs, no terminal trashing
  #   2. An assistant message containing control characters (\x00\x07) and carriage returns displays as clean text in the AgentView scrollback, the turn-content modal (Enter on the turn), and when copied to the clipboard
  #   3. A thinking chunk with ANSI cursor-movement sequences (neofetch-style) shows readable plain text under the [Thinking] header in the scrollback
  #   4. A tool call whose argument strings contain escape sequences (e.g. file_path or command with \x1b codes) shows clean argument text in the tool-card header line
  #   5. An Edit/Write tool with escape sequences in its input shows a clean colored diff inline in the tool card AND in the turn-content modal, while the diff's elision hint rows ("... (N lines)") still render correctly
  #   6. A provider error chunk (StreamChunk::Error) with ANSI in the error text shows clean text both in the inline "API Error: ..." scrollback line and in the centred ErrorDialog modal
  #   7. A HITL prompt (request_user_input) whose question/options carry control characters shows clean text in the inline pause-style prompt rows (header, question, option labels + descriptions)
  #   8. A role set via /role with a control character shows clean single-line text in the RoleBanner under the SessionHeader
  #   9. A session notice (Action::EmitSessionNotice) whose text carries escape sequences lands as a clean line in the originating session's scrollback
  #   10. A changed file whose path and diff content contain ANSI codes shows clean rows in the Changed Files dual-pane (file list + diff pane)
  #   11. A checkpoint with a name containing control characters and a diff with tabs shows clean rows in the Checkpoints three-pane (checkpoint list, files, diff) and in the restore dialog
  #   12. While the Changed Files diff stage is in flight, the shared loading dialog's spinner label (which embeds the file path) shows clean text even when the path carries control characters
  #   13. A blocklist rule whose pattern/reason/guidance contain escape sequences shows clean rows in the BlocklistView list pane and details pane
  #   14. Plain text with no escape sequences or control characters passes through sanitization byte-for-byte unchanged in every sink (no visual difference after the refactor)
  #
  # ASSUMPTIONS:
  #   1. The sanitize_for_terminal behaviour (ANSI regex + control-char set + tab/CR handling) is already correct and frozen — this card is a STRUCTURAL refactor (where sanitization happens), not a behaviour change. If behaviour needs to change, that is a separate card.
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to see clean, terminal-safe output in every TUI view (AgentView, BoardView, Changed Files, Checkpoints, dialogs, prompts)
    So that no view can display raw ANSI sequences, control characters, tabs or carriage returns from LLM, backend, filesystem or user data

  # ============================================================
  # STRUCTURAL — where the sanitizer lives (rules 1, 2)
  # ============================================================
  @unit
  @source-shape
  @tui
  Scenario: The canonical sanitizer lives in ONE crate-level module
    Given the fspec-tui crate source tree
    When I locate the terminal-output sanitizer implementation
    Then the canonical sanitize_for_terminal implementation lives in a crate-level terminal::sanitize module
    And the crate root re-exports sanitize_for_terminal for in-crate and external importers
    And the old store/agent_view/sanitize.rs module no longer exists
    And no view, store, or component module defines its own ANSI or control-character stripping

  @unit
  @tui
  Scenario: The sanitizer is idempotent and control-char-free on arbitrary input
    Given any arbitrary text (including lone ESC bytes, partial escape sequences, tabs, CRs and NULs)
    When the text is passed through sanitize_for_terminal
    Then the result contains no ESC byte and no forbidden control character
    And passing the result through the sanitizer a second time changes nothing

  @unit
  @agent-view
  @scrollback
  Scenario: Assistant text with control characters displays clean in the scrollback
  # ============================================================
  # AgentView scrollback ingress (rule 3)
  # ============================================================
    Given a session recording a StreamChunk::Text whose payload contains ANSI color codes, a tab, a carriage return and a bell character
    When the chunk is recorded into the session scrollback
    Then the stored chunk text has the ANSI sequences removed, the tab replaced with two spaces, the carriage return and bell character removed, and newlines preserved

  @unit
  @agent-view
  @scrollback
  Scenario: Thinking chunks with ANSI cursor movement display as readable plain text
    Given a session recording a StreamChunk::Thinking payload containing neofetch-style cursor-movement and color sequences
    When the chunk is recorded into the session scrollback
    Then the stored thinking chunk text is plain readable text with every escape sequence removed

  @unit
  @agent-view
  Scenario: Tool call argument strings with escape sequences display clean in the tool-card header
    Given a session recording a StreamChunk::ToolCall whose input JSON carries escape sequences inside argument strings
    When the tool-call card is pushed to the scrollback
    Then the card header line (tool name + argument display) contains no escape sequences or control characters

  @unit
  @diff-display
  Scenario: Edit/Write tool diff shows clean lines while elision hints still render
    Given an Edit or Write tool whose input strings (and surrounding context file lines) contain escape sequences and tabs
    When the matching tool result produces the diff card
    Then every diff content line in the inline card and the full-text modal variant is clean
    And the diff elision hint rows (such as "... (N lines)") render exactly as before the refactor

  @unit
  @agent-view
  Scenario: Provider error text displays clean in the scrollback error line
    Given a session recording a StreamChunk::Error whose error string contains ANSI codes
    When the error is recorded into the scrollback
    Then the stored "API Error: ..." chunk text is free of escape sequences and control characters

  @unit
  @agent-view
  Scenario: User input, notifications and supervisor messages display clean in the scrollback
    Given a session that records a StreamChunk::UserInput, a StreamChunk::UserNotification and a StreamChunk::IncomingMessage each carrying escape sequences or control characters
    When the chunks are recorded into the session scrollback
    Then all three stored chunk texts are free of escape sequences and control characters

  @integration
  @agent-view
  @scrollback
  @text-selection
  Scenario: Hostile assistant text stays clean through the turn modal and clipboard copy
    Given an agent session whose assistant message contains control characters and carriage returns
    When I open the turn-content modal for that turn and select-copy the same text in the scrollback
    Then the modal displays the text without control characters or carriage returns
    And the copied text carries no escape bytes or forbidden control characters
    And the LLM-facing tool output path still receives the raw unsanitized text

  @integration
  @board
  Scenario: Work unit titles with ANSI codes display plain in the board
  # ============================================================
  # BoardView (rule 4)
  # ============================================================
    Given a work unit whose title contains ANSI color codes and a tab
    When the work-units snapshot is loaded into the board store
    Then the board card label, the details-strip title row and the work-unit search dialog rows all show the plain text with no escape glyphs
    And the terminal display is not corrupted by the escape sequences

  @integration
  @board
  Scenario: Work unit descriptions and attachment paths display clean in the details strip
    Given a work unit whose description and attachment paths contain control characters
    When the board renders the details strip for that selected work unit
    Then the description rows and the attachments row show clean text
    And the selection-copy reader of the strip rows returns the same clean text

  @unit
  @agent-view
  Scenario: HITL prompt questions and options display clean
  # ============================================================
  # Prompts / chrome (rule 5)
  # ============================================================
    Given a HITL prompt request whose question header, question text and option labels/descriptions carry control characters
    When the request is stored for the session
    Then the stored prompt state holds clean text for every header, question, option label and option description

  @unit
  @agent-view
  Scenario: Pause prompts, exec-stdin commands and role text display clean
    Given a pause state prompt, an exec-stdin command display string and a session role text that each contain escape sequences or control characters
    When each is stored via its store setter
    Then the pause prompt, the exec-stdin prompt text and the role banner text are stored clean

  @unit
  @agent-view
  Scenario: A session notice with escape sequences lands clean in the scrollback
  # ============================================================
  # Session notices (rule 2)
  # ============================================================
    Given an EmitSessionNotice action whose text carries escape sequences and control characters
    When the notice is applied to the originating session
    Then the stored scrollback line is free of escape sequences and control characters

  @integration
  @diff-display
  Scenario: Changed files with ANSI in paths and diff content display clean
  # ============================================================
  # Mode views (rule 7)
  # ============================================================
    Given a changed-file snapshot whose file paths and diff lines contain ANSI codes
    When the Changed Files view loads the files and a diff
    Then the file-list rows and the diff-pane rows display clean text
    And the terminal display is not corrupted by the escape sequences

  @integration
  @checkpoints
  Scenario: Checkpoint names and diffs with control characters display clean
    Given a checkpoint whose name contains control characters and a file diff that contains tabs
    When the Checkpoints view loads the checkpoint list, files and diff
    Then the checkpoint label row, the file rows and the diff rows display clean text
    And each tab renders as two spaces

  @integration
  Scenario: Resume session rows with hostile project names display clean
    Given a session list whose session names and project paths contain escape sequences
    When the Resume Session view loads the sessions
    Then every rendered session row shows clean text

  @integration
  Scenario: Blocklist rules with escape sequences display clean
    Given a blocklist whose rule pattern, reason and guidance fields contain escape sequences
    When the Blocklist view loads the rules
    Then the list-pane rows and the details-pane fields display clean text

  @integration
  Scenario: Model selector rows with hostile display names display clean
    Given a provider/model snapshot whose display names contain control characters
    When the Model Selector view loads the providers
    Then the provider and model rows display clean text

  @unit
  @dialog
  Scenario: Error dialogs display clean provider error text
  # ============================================================
  # Dialogs (rule 6)
  # ============================================================
    Given a provider error string containing ANSI codes
    When the error dialog is created from that string
    Then the dialog renders the message without escape sequences or control characters

  @unit
  @dialog
  Scenario: Loading dialog labels with hostile file paths display clean
    Given a loading dialog whose spinner label embeds a file path containing control characters
    When the loading dialog renders its spinner line
    Then the label displays clean text

  @regression
  @unit
  @parity
  Scenario: Plain text passes through every sink byte-for-byte unchanged
  # ============================================================
  # Regression guard (rule 9 + assumption 1)
  # ============================================================
    Given plain text containing no escape sequences, tabs or control characters (only newlines and printable chars)
    When it is ingested through every sanitized boundary (chunk recorder, board store, prompt setters, view setters, dialog constructors)
    Then every stored/displayed copy of the text is byte-for-byte identical to the input
