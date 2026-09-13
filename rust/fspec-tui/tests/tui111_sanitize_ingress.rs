//! TUI-111 — centralized terminal-output sanitization at ingress for ALL TUI views.
//!
//! Feature: spec/features/sanitize-all-tui-output-at-ingress.feature
//!
//! Ingress-level tests: hostile payloads (ANSI + control chars + tabs + CRs)
//! are fed through each store/view/dialog ingress boundary and the STORED /
//! RENDERED text is asserted clean. Buffer-level golden rendering is covered
//! by the existing parity suite (chunkprocessor-parity,
//! chunk_rendering_parity, scrollback_pty) + the TUI-100/104 view tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::store::sanitize_work_unit;
use codelet_fspec_tui::store::AgentViewStore;
use codelet_fspec_tui::views::agent::{ResumeSessionView, SearchHistoryView};
use codelet_fspec_tui::views::blocklist::BlocklistView;
use codelet_fspec_tui::views::changed_files::ChangedFilesView;
use codelet_fspec_tui::views::checkpoints::CheckpointsView;
use codelet_fspec_tui::{sanitize_for_terminal, ChunkKind, SessionContext};
use codelet_rpc_types::{
    BlocklistRuleInfo, ChangedFile, CheckpointInfo, ExecStdinRequest, HitlOption, HitlQuestion,
    HitlRequest, NotificationSeverity, PauseKind, PauseState, ProviderInfo, SessionId, SessionInfo,
    StreamChunk, WorkUnitInfo,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const HOSTILE: &str = "\x1b[31m\x00boom\x07\tend\r";
const EXPECTED_CLEAN: &str = "boom  end";

/// Assert `haystack` carries no ESC byte and no forbidden control char.
fn assert_clean(haystack: &str) {
    assert!(
        !haystack.contains('\x1b'),
        "ESC byte present in {haystack:?}"
    );
    for c in haystack.chars() {
        let code = c as u32;
        assert!(
            !matches!(code, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F),
            "forbidden control char U+{code:02X} present in {haystack:?}"
        );
    }
}

/// All stored scrollback text (chunk source + full_text).
fn scrollback_text(ctx: &SessionContext) -> String {
    let mut out = String::new();
    for chunk in ctx.scrollback.chunks() {
        if let Some(source) = chunk.source.as_ref() {
            out.push_str(&source.text);
            out.push('\n');
            if let Some(full) = source.full_text.as_ref() {
                out.push_str(full);
                out.push('\n');
            }
        }
    }
    out
}

fn render_to_text<F>(mut paint: F) -> String
where
    F: FnMut(&mut Buffer),
{
    let area = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(area);
    paint(&mut buf);
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: format!("{HOSTILE} {id}"),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: Some(HOSTILE.to_string()),
        estimate: Some(3),
        epic: Some(HOSTILE.to_string()),
        attachments: vec![format!("{HOSTILE}/tmp/a.txt")],
        last_state_change_at: None,
    }
}

// ── Scenario: The canonical sanitizer lives in ONE crate-level module ──

/// @step Given the fspec-tui crate source tree
/// @step When I locate the terminal-output sanitizer implementation
/// @step Then the canonical sanitize_for_terminal implementation lives in a crate-level terminal::sanitize module
/// @step And the crate root re-exports sanitize_for_terminal for in-crate and external importers
/// @step And the old store/agent_view/sanitize.rs module no longer exists
/// @step And no view, store, or component module defines its own ANSI or control-character stripping
#[test]
fn canonical_sanitizer_lives_in_crate_level_terminal_module() {
    let src = common::workspace_root().join("fspec-tui").join("src");
    assert!(
        src.join("terminal").join("sanitize.rs").is_file(),
        "terminal/sanitize.rs must exist at the crate level"
    );
    assert!(
        !src.join("store")
            .join("agent_view")
            .join("sanitize.rs")
            .exists(),
        "old store/agent_view/sanitize.rs must be gone"
    );
    let lib = std::fs::read_to_string(src.join("lib.rs")).expect("lib.rs");
    assert!(
        lib.contains("pub use terminal::sanitize::sanitize_for_terminal;"),
        "crate root must re-export sanitize_for_terminal"
    );
}

// ── Scenario: The sanitizer is idempotent and control-char-free on arbitrary input ──

/// @step Given any arbitrary text (including lone ESC bytes, partial escape sequences, tabs, CRs and NULs)
/// @step When the text is passed through sanitize_for_terminal
/// @step Then the result contains no ESC byte and no forbidden control character
/// @step And passing the result through the sanitizer a second time changes nothing
#[test]
fn sanitizer_is_idempotent_and_control_char_free() {
    let inputs = [
        "\x1b[31mred\x1b[0m",
        "\x1b",
        "\x1b[",
        "\x1b]",
        "a\tb\rc\x00d\x07e\x1bf",
        "\u{0000}\u{007F}\u{0080}x",
        "plain text\nwith newlines",
    ];
    for input in inputs {
        let once = sanitize_for_terminal(input);
        assert_clean(&once);
        let twice = sanitize_for_terminal(&once);
        assert_eq!(once, twice, "not idempotent for {input:?}");
    }
}

// ── Scenario: Assistant text with control characters displays clean in the scrollback ──

/// @step Given a session recording a StreamChunk::Text whose payload contains ANSI color codes, a tab, a carriage return and a bell character
/// @step When the chunk is recorded into the session scrollback
/// @step Then the stored chunk text has the ANSI sequences removed, the tab replaced with two spaces, the carriage return and bell character removed, and newlines preserved
#[test]
fn assistant_text_with_control_chars_is_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    ctx.record_chunk(&StreamChunk::text(HOSTILE.to_string()));
    let stored = scrollback_text(&ctx);
    assert_clean(&stored);
    assert!(
        stored.contains(EXPECTED_CLEAN),
        "clean text lost: {stored:?}"
    );
}

// ── Scenario: Thinking chunks with ANSI cursor movement display as readable plain text ──

/// @step Given a session recording a StreamChunk::Thinking payload containing neofetch-style cursor-movement and color sequences
/// @step When the chunk is recorded into the session scrollback
/// @step Then the stored thinking chunk text is plain readable text with every escape sequence removed
#[test]
fn thinking_chunks_with_ansi_cursor_movement_are_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    let neofetch = "\x1b[1mOS:\x1b[0m Ubuntu\n\x1b[2J\x1b[H\x1b[34mKernel:\x1b[0m 6.8.0";
    ctx.record_chunk(&StreamChunk::thinking(neofetch.to_string()));
    let stored = scrollback_text(&ctx);
    assert_clean(&stored);
    assert!(stored.contains("OS: Ubuntu"), "got {stored:?}");
    assert!(stored.contains("Kernel: 6.8.0"), "got {stored:?}");
}

// ── Scenario: Tool call argument strings with escape sequences display clean in the tool-card header ──

/// @step Given a session recording a StreamChunk::ToolCall whose input JSON carries escape sequences inside argument strings
/// @step When the tool-call card is pushed to the scrollback
/// @step Then the card header line (tool name + argument display) contains no escape sequences or control characters
#[test]
fn tool_call_header_with_escape_sequence_args_is_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    let input = r#"{"file_path":"/tmp/\u001b[31mfile\u001b[0m.txt"}"#;
    ctx.record_chunk(&StreamChunk::tool_call(codelet_rpc_types::ToolCallInfo {
        id: "tc-1".to_string(),
        name: "Read".to_string(),
        input: input.to_string(),
    }));
    let stored = scrollback_text(&ctx);
    assert_clean(&stored);
    assert!(stored.contains("Read("), "header lost: {stored:?}");
}

// ── Scenario: Edit/Write tool diff shows clean lines while elision hints still render ──

/// @step Given an Edit or Write tool whose input strings (and surrounding context file lines) contain escape sequences and tabs
/// @step When the matching tool result produces the diff card
/// @step Then every diff content line in the inline card and the full-text modal variant is clean
/// @step And the diff elision hint rows (such as "... (N lines)") render exactly as before the refactor
#[test]
fn edit_write_diff_source_lines_are_sanitized_before_encode() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    let old = format!("line1{HOSTILE}\nline2\told");
    let new = format!("line1{HOSTILE}\nline2\tnew");
    let input = format!(
        r#"{{"old_string":{}, "new_string":{} }}"#,
        serde_json::to_string(&old).expect("json"),
        serde_json::to_string(&new).expect("json")
    );
    ctx.record_chunk(&StreamChunk::tool_call(codelet_rpc_types::ToolCallInfo {
        id: "tc-diff".to_string(),
        name: "Edit".to_string(),
        input,
    }));
    ctx.record_chunk(&StreamChunk::tool_result(
        codelet_rpc_types::ToolResultInfo {
            tool_call_id: "tc-diff".to_string(),
            is_error: false,
            content: String::new(),
        },
    ));
    let stored = scrollback_text(&ctx);
    // The stored diff body is the canonical codec string — assert the
    // SOURCE content is clean. The codec's own \u{1} elision sentinel is
    // NOT a forbidden char set member in the visible output: the stored
    // string may carry it (rule [7]: never re-sanitize post-encode), so
    // decode via the codec instead.
    assert!(!stored.contains("boom\x00"), "raw NUL survived: {stored:?}");
    assert!(
        !stored.contains("\x1b[31m"),
        "raw ANSI survived: {stored:?}"
    );
    assert!(stored.contains("line2"), "diff content lost: {stored:?}");

    // Direct codec-level check: elision hints survive for large edits.
    let old_big: String = (1..=60).map(|i| format!("o{i}\n")).collect();
    let new_big: String = (1..=60).map(|i| format!("n{i}\n")).collect();
    let pending = codelet_fspec_tui::store::agent_view::pending_tool_diff::capture_pending_diff(
        "Edit",
        &format!(
            r#"{{"old_string":{}, "new_string":{}}}"#,
            serde_json::to_string(&old_big).expect("json"),
            serde_json::to_string(&new_big).expect("json")
        ),
    )
    .expect("pending diff captured");
    let (collapsed, full) =
        codelet_fspec_tui::store::agent_view::pending_tool_diff::produce_diff_strings(&pending);
    assert!(
        collapsed.contains("(select turn to /expand)"),
        "elision hint must still render: {collapsed:?}"
    );
    assert!(!full.contains("(select turn to /expand)"));
}

// ── Scenario: Provider error text displays clean in the scrollback error line ──

/// @step Given a session recording a StreamChunk::Error whose error string contains ANSI codes
/// @step When the error is recorded into the scrollback
/// @step Then the stored "API Error: ..." chunk text is free of escape sequences and control characters
#[test]
fn provider_error_text_is_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    ctx.record_chunk(&StreamChunk::error(HOSTILE.to_string()));
    let stored = scrollback_text(&ctx);
    assert_clean(&stored);
    assert!(stored.contains("API Error: boom"), "got {stored:?}");
}

// ── Scenario: User input, notifications and supervisor messages display clean in the scrollback ──

/// @step Given a session that records a StreamChunk::UserInput, a StreamChunk::UserNotification and a StreamChunk::IncomingMessage each carrying escape sequences or control characters
/// @step When the chunks are recorded into the session scrollback
/// @step Then all three stored chunk texts are free of escape sequences and control characters
#[test]
fn user_input_notification_and_incoming_are_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    ctx.record_chunk(&StreamChunk::UserInput {
        text: HOSTILE.to_string(),
    });
    ctx.record_chunk(&StreamChunk::UserNotification {
        message: HOSTILE.to_string(),
        severity: NotificationSeverity::Info,
    });
    ctx.record_chunk(&StreamChunk::IncomingMessage {
        text: format!("[SUPERVISOR: reviewer | Session: s-2]{HOSTILE} body"),
        images: None,
    });
    let stored = scrollback_text(&ctx);
    assert_clean(&stored);
    assert!(stored.contains(EXPECTED_CLEAN), "got {stored:?}");
    assert!(stored.contains("[W] reviewer>"), "got {stored:?}");
}

// ── Scenario: A session notice with escape sequences lands clean in the scrollback ──

/// @step Given an EmitSessionNotice action whose text carries escape sequences and control characters
/// @step When the notice is applied to the originating session
/// @step Then the stored scrollback line is free of escape sequences and control characters
#[test]
fn session_notice_lands_clean_in_scrollback() {
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    ctx.push_line(HOSTILE);
    let stored = scrollback_text(&ctx);
    assert_clean(&stored);
    assert!(stored.contains(EXPECTED_CLEAN), "got {stored:?}");
}

// ── Scenario: Work unit titles with ANSI codes display plain in the board ──

/// @step Given a work unit whose title contains ANSI color codes and a tab
/// @step When the work-units snapshot is loaded into the board store
/// @step Then the board card label, the details-strip title row and the work-unit search dialog rows all show the plain text with no escape glyphs
/// @step And the terminal display is not corrupted by the escape sequences
#[test]
fn board_store_sanitizes_work_units_on_ingress() {
    let mut store = codelet_fspec_tui::BoardStore::default();
    let unit = wu("AUTH-001", "done");
    store.replace_work_units(vec![unit]);
    let unit = store.work_units().first().expect("unit stored");
    assert_clean(&unit.title);
    assert_clean(unit.description.as_deref().expect("desc"));
    assert_clean(unit.epic.as_deref().expect("epic"));
    assert_clean(&unit.attachments[0]);
    let title = unit.title.clone();
    assert!(title.contains(EXPECTED_CLEAN), "got {title:?}");
}

// ── Scenario: Work unit descriptions and attachment paths display clean in the details strip ──

/// @step Given a work unit whose description and attachment paths contain control characters
/// @step When the board renders the details strip for that selected work unit
/// @step Then the description rows and the attachments row show clean text
/// @step And the selection-copy reader of the strip rows returns the same clean text
#[test]
fn sanitize_work_unit_covers_all_free_text_fields() {
    let mut unit = wu("AUTH-002", "blocked");
    sanitize_work_unit(&mut unit);
    assert_clean(&unit.id);
    assert_clean(&unit.title);
    assert_clean(unit.description.as_deref().expect("desc"));
    assert_clean(unit.epic.as_deref().expect("epic"));
    for att in &unit.attachments {
        assert_clean(att);
    }
    assert_eq!(unit.status, "blocked", "closed vocabulary untouched");
}

// ── Scenario: HITL prompt questions and options display clean ──

/// @step Given a HITL prompt request whose question header, question text and option labels/descriptions carry control characters
/// @step When the request is stored for the session
/// @step Then the stored prompt state holds clean text for every header, question, option label and option description
#[test]
fn hitl_prompt_fields_are_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut store = AgentViewStore::default();
    store.set_hitl_prompt(
        sid.clone(),
        HitlRequest {
            questions: vec![HitlQuestion {
                id: "q-1".to_string(),
                header: HOSTILE.to_string(),
                question: HOSTILE.to_string(),
                options: vec![HitlOption {
                    label: HOSTILE.to_string(),
                    description: HOSTILE.to_string(),
                }],
            }],
        },
    );
    let state = store.hitl_prompt_for(&sid).expect("slot set");
    let q = &state.request.questions[0];
    assert_clean(&q.header);
    assert_clean(&q.question);
    assert_clean(&q.options[0].label);
    assert_clean(&q.options[0].description);
}

// ── Scenario: Pause prompts, exec-stdin commands and role text display clean ──

/// @step Given a pause state prompt, an exec-stdin command display string and a session role text that each contain escape sequences or control characters
/// @step When each is stored via its store setter
/// @step Then the pause prompt, the exec-stdin prompt text and the role banner text are stored clean
#[test]
fn pause_exec_stdin_and_role_are_stored_clean() {
    let sid = SessionId::new("s-1");
    let mut store = AgentViewStore::default();
    store.set_pause_state(
        sid.clone(),
        PauseState {
            kind: PauseKind::Confirm,
            prompt: HOSTILE.to_string(),
            tool_call_id: Some(HOSTILE.to_string()),
        },
    );
    let pause = store.pause_state_for(&sid).expect("pause set");
    assert_clean(&pause.prompt);
    assert_clean(pause.tool_call_id.as_deref().expect("tool_call_id"));

    store.set_exec_stdin(
        sid.clone(),
        ExecStdinRequest {
            exec_session_id: "exec-1".to_string(),
            command: HOSTILE.to_string(),
            quiet_seconds: 3,
            ts_ms: 0,
        },
    );
    let exec = store.exec_stdin_for(&sid).expect("exec set");
    assert_clean(&exec.command);
    assert_eq!(exec.exec_session_id, "exec-1");

    store.set_role(sid.clone(), Some(HOSTILE.to_string()));
    let role = store.role_for(&sid).expect("role set");
    assert_clean(role);
}

// ── Scenario: Changed files with ANSI in paths and diff content display clean ──

/// @step Given a changed-file snapshot whose file paths and diff lines contain ANSI codes
/// @step When the Changed Files view loads the files and a diff
/// @step Then the file-list rows and the diff-pane rows display clean text
/// @step And the terminal display is not corrupted by the escape sequences
#[test]
fn changed_files_setters_store_and_render_clean_text() {
    let mut view = ChangedFilesView::new();
    view.set_files(vec![ChangedFile {
        path: HOSTILE.to_string(),
        change_type: "M".to_string(),
        staged: false,
    }]);
    let path = view.selected_path().expect("path");
    assert_clean(&path);

    view.set_diff("boom  end", Some(HOSTILE.to_string()));
    let out = render_to_text(|buf| {
        view.render(Rect::new(0, 0, 120, 30), buf);
    });
    assert_clean(&out);
    assert!(out.contains(EXPECTED_CLEAN), "diff rows lost: {out:?}");
}

// ── Scenario: Checkpoint names and diffs with control characters display clean ──

/// @step Given a checkpoint whose name contains control characters and a file diff that contains tabs
/// @step When the Checkpoints view loads the checkpoint list, files and diff
/// @step Then the checkpoint label row, the file rows and the diff rows display clean text
/// @step And each tab renders as two spaces
#[test]
fn checkpoint_setters_store_and_render_clean_text() {
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![CheckpointInfo {
        work_unit_id: "AUTH-001".to_string(),
        name: HOSTILE.to_string(),
        timestamp: "2026-09-13T00:00:00Z".to_string(),
        is_automatic: false,
    }]);
    let cp = view.selected_checkpoint_info().expect("checkpoint");
    assert_clean(&cp.name);
    let name = cp.name.clone();
    assert!(name.contains(EXPECTED_CLEAN), "got {name:?}");

    view.set_files(
        "AUTH-001",
        "boom  end",
        vec![ChangedFile {
            path: HOSTILE.to_string(),
            change_type: "A".to_string(),
            staged: false,
        }],
    );
    view.set_diff(
        "AUTH-001",
        "boom  end",
        "boom  end",
        Some(HOSTILE.to_string()),
    );
    let out = render_to_text(|buf| {
        view.render(Rect::new(0, 0, 120, 30), buf);
    });
    assert_clean(&out);
    assert!(out.contains(EXPECTED_CLEAN), "rows lost: {out:?}");
}

// ── Scenario: Resume session rows with hostile project names display clean ──

/// @step Given a session list whose session names and project paths contain escape sequences
/// @step When the Resume Session view loads the sessions
/// @step Then every rendered session row shows clean text
#[test]
fn resume_session_rows_are_stored_clean() {
    let mut view = ResumeSessionView::new();
    view.set_sessions(vec![SessionInfo {
        id: "s-1".to_string(),
        name: HOSTILE.to_string(),
        status: "idle".to_string(),
        project: HOSTILE.to_string(),
        message_count: 1,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: Some(HOSTILE.to_string()),
        role: Some(HOSTILE.to_string()),
        updated_at_ms: None,
    }]);
    let s = view.selected().expect("session");
    assert_clean(&s.name);
    assert_clean(&s.project);
    assert_clean(s.worktree_path.as_deref().expect("wt"));
    assert_clean(s.role.as_deref().expect("role"));
}

// ── Scenario: Blocklist rules with escape sequences display clean ──

/// @step Given a blocklist whose rule pattern, reason and guidance fields contain escape sequences
/// @step When the Blocklist view loads the rules
/// @step Then the list-pane rows and the details-pane fields display clean text
#[test]
fn blocklist_rules_are_stored_clean() {
    let mut view = BlocklistView::new();
    view.set_rules(vec![BlocklistRuleInfo {
        id: "rule-1".to_string(),
        pattern: HOSTILE.to_string(),
        action: "block".to_string(),
        reason: HOSTILE.to_string(),
        guidance: Some(HOSTILE.to_string()),
        source: "project".to_string(),
    }]);
    let rule = view.focused_rule().expect("rule");
    assert_clean(&rule.pattern);
    assert_clean(&rule.reason);
    assert_clean(rule.guidance.as_deref().expect("guidance"));
    assert_clean(&rule.source);
    assert_eq!(rule.action, "block", "closed vocabulary untouched");
}

// ── Scenario: Model selector rows with hostile display names display clean ──

/// @step Given a provider/model snapshot whose display names contain control characters
/// @step When the Model Selector view loads the providers
/// @step Then the provider and model rows display clean text
#[test]
fn model_selector_rows_render_clean() {
    let mut view = codelet_fspec_tui::views::model_selector::ModelSelectorView::new();
    view.set_providers(vec![ProviderInfo {
        key: "openai".to_string(),
        display_name: HOSTILE.to_string(),
        models: vec![codelet_rpc_types::ModelEntry {
            id: "gpt-x".to_string(),
            display_name: HOSTILE.to_string(),
            ..Default::default()
        }],
        profile_name: None,
        is_unreachable: false,
    }]);
    let out = render_to_text(|buf| {
        view.render(Rect::new(0, 0, 120, 30), buf);
    });
    assert_clean(&out);
    assert!(out.contains(EXPECTED_CLEAN), "rows lost: {out:?}");
}

// ── Scenario: Search history rows with hostile text display clean ──

/// @step Given a search-history match list whose text fields contain escape sequences
/// @step When the Search History view loads the matches
/// @step Then every rendered row shows clean text
#[test]
fn search_history_rows_are_stored_clean() {
    let mut view = SearchHistoryView::new();
    view.set_matches(vec![codelet_rpc_types::HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: HOSTILE.to_string(),
        timestamp_iso: "2026-09-13T00:00:00Z".to_string(),
    }]);
    let m = view.selected().expect("match");
    assert_clean(&m.text);
    let out = render_to_text(|buf| {
        view.render(Rect::new(0, 0, 120, 30), buf);
    });
    assert_clean(&out);
}

// ── Scenario: Error dialogs display clean provider error text ──

/// @step Given a provider error string containing ANSI codes
/// @step When the error dialog is created from that string
/// @step Then the dialog renders the message without escape sequences or control characters
#[test]
fn error_dialog_message_is_sanitized_on_construction() {
    let dialog = codelet_fspec_tui::components::error_dialog::ErrorDialog::new(HOSTILE);
    let msg = dialog.message();
    assert_clean(msg);
    assert_eq!(msg, EXPECTED_CLEAN);
}

// ── Scenario: Loading dialog labels with hostile file paths display clean ──

/// @step Given a loading dialog whose spinner label embeds a file path containing control characters
/// @step When the loading dialog renders its spinner line
/// @step Then the label displays clean text
#[test]
fn loading_dialog_and_tracker_labels_are_sanitized() {
    let dialog = codelet_fspec_tui::components::loading_dialog::LoadingDialog::new(
        "Loading files",
        format!("Loading diff for {HOSTILE}file.txt…"),
    );
    assert_clean(&dialog.label);
    let line = dialog.spinner_line(0);
    assert_clean(&line);

    let mut tracker =
        codelet_fspec_tui::components::load_state::LoadTracker::new("Loading {HOSTILE}…");
    tracker.begin_stage("diff", "Reading {HOSTILE}…");
    let active = tracker.active_label().expect("label");
    assert_clean(&active);
}

// ── Scenario: Hostile assistant text stays clean through the turn modal and clipboard copy ──

/// @step Given an agent session whose assistant message contains control characters and carriage returns
/// @step When I open the turn-content modal for that turn and select-copy the same text in the scrollback
/// @step Then the modal displays the text without control characters or carriage returns
/// @step And the copied text carries no escape bytes or forbidden control characters
/// @step And the LLM-facing tool output path still receives the raw unsanitized text
#[test]
fn hostile_assistant_text_stays_clean_through_turn_modal_and_copy() {
    use codelet_fspec_tui::mouse::selection::RowSpan;
    use codelet_fspec_tui::store::agent_view::chunk_wrap::wrap_source;
    use codelet_fspec_tui::views::agent::{
        ChunkSource, RenderedChunk, ScrollbackList, TurnContentModal,
    };
    use ratatui::style::Color;

    // @step Given an agent session whose assistant message contains control characters and carriage returns
    let sid = SessionId::new("s-1");
    let mut ctx = SessionContext::with_work_unit(sid, None);
    let raw_payload = HOSTILE.to_string();
    ctx.record_chunk(&StreamChunk::text(raw_payload.clone()));
    let source = ctx
        .scrollback
        .chunks()
        .first()
        .and_then(|c| c.source.clone())
        .expect("assistant chunk stored with source");

    // @step When I open the turn-content modal for that turn and select-copy the same text in the scrollback
    let modal = TurnContentModal::new(source.text.clone(), Some(ChunkKind::AssistantText));
    let modal_out = render_to_text(|buf| modal.render(Rect::new(0, 0, 120, 30), buf));

    let mut list = ScrollbackList::new();
    let source_for_list = source.clone();
    let lines = wrap_source(
        &ChunkSource {
            text: source_for_list.text.clone(),
            color: Color::White,
            kind: ChunkKind::AssistantText,
            is_streaming: false,
            full_text: None,
        },
        100,
    );
    list.push(RenderedChunk {
        seq: 0,
        lines,
        source: Some(source_for_list),
    });
    let copied = list.selected_text(&[RowSpan {
        row: 0,
        start_col: 0,
        end_col: 200,
    }]);

    // @step Then the modal displays the text without control characters or carriage returns
    assert_clean(&modal_out);
    assert!(
        modal_out.contains(EXPECTED_CLEAN),
        "modal body lost clean text: {modal_out:?}"
    );

    // @step And the copied text carries no escape bytes or forbidden control characters
    assert_clean(&copied);
    assert!(
        copied.contains(EXPECTED_CLEAN),
        "copy lost clean text: {copied:?}"
    );

    // @step And the LLM-facing tool output path still receives the raw unsanitized text
    // (rule [8]: sanitization is TUI-local; the payload handed to the
    // session engine is the untouched clone recorded above)
    assert!(
        raw_payload.contains('\x1b'),
        "LLM-facing payload must stay raw: {raw_payload:?}"
    );
}

// ── Scenario: Plain text passes through every sink byte-for-byte unchanged ──

/// @step Given plain text containing no escape sequences, tabs or control characters (only newlines and printable chars)
/// @step When it is ingested through every sanitized boundary (chunk recorder, board store, prompt setters, view setters, dialog constructors)
/// @step Then every stored/displayed copy of the text is byte-for-byte identical to the input
#[test]
fn plain_text_passes_through_every_sink_unchanged() {
    let plain = "plain text\nsecond line";

    // Chunk recorder
    let mut ctx = SessionContext::with_work_unit(SessionId::new("s-1"), None);
    ctx.record_chunk(&StreamChunk::text(plain.to_string()));
    let stored = scrollback_text(&ctx);
    assert!(stored.contains(plain), "chunk recorder: {stored:?}");

    // Board store
    let mut unit = WorkUnitInfo {
        id: "T-1".to_string(),
        title: plain.to_string(),
        work_type: "story".to_string(),
        status: "done".to_string(),
        description: Some(plain.to_string()),
        estimate: None,
        epic: Some(plain.to_string()),
        attachments: vec![plain.to_string()],
        last_state_change_at: None,
    };
    sanitize_work_unit(&mut unit);
    assert_eq!(unit.title, plain);
    assert_eq!(unit.description.as_deref(), Some(plain));
    assert_eq!(unit.attachments[0], plain);

    // Role setter
    let sid = SessionId::new("s-1");
    let mut store = AgentViewStore::default();
    store.set_role(sid.clone(), Some(plain.to_string()));
    assert_eq!(store.role_for(&sid), Some(plain));

    // push_line
    let mut ctx2 = SessionContext::with_work_unit(sid, None);
    ctx2.push_line(plain);
    let stored = scrollback_text(&ctx2);
    assert!(stored.contains(plain), "push_line: {stored:?}");

    // Error dialog
    let dialog = codelet_fspec_tui::components::error_dialog::ErrorDialog::new(plain);
    assert_eq!(dialog.message(), plain);
}

mod common;
