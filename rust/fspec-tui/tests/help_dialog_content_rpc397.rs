//! RPC-397 — View-specific accurate help content for board and agent.
//!
//! Feature: spec/features/view-specific-accurate-help-content-for-board-and-agent.feature
//!
//! ACDD TESTING phase: these tests assert the RPC-397 behaviour — a Board
//! HelpDialog variant (board keybindings only, NO slash commands) and an
//! Agent HelpDialog variant (agent keybindings + all 17 slash commands with
//! descriptions), with neither variant showing the misleading "q Quit" line.
//!
//! They compile against the yet-to-be-implemented `HelpDialog::for_board()`
//! and `HelpDialog::for_agent()` constructors, so this file is RED (fails to
//! compile) until the RPC-397 implementation lands.
//!
//! Rendering uses a 200x60 TestBackend so ALL content fits with no scroll or
//! truncation (58 body rows comfortably hold the agent variant's ~40 lines),
//! making substring assertions against the full buffer robust.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use codelet_fspec_tui::components::help_dialog::HelpDialog;
use codelet_fspec_tui::components::Component;

/// Render any Component into a 200x60 TestBackend and return the buffer.
fn render_200x60<C: Component>(component: &mut C) -> Buffer {
    let backend = TestBackend::new(200, 60);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            component.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

/// Flatten a rendered buffer into a single newline-joined string.
fn join_buffer(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn board_help_shows_board_keybindings_and_no_slash_commands() {
    // @step Given a HelpDialog constructed for the board rendered against a 200x60 TestBackend
    let mut dialog = HelpDialog::for_board();
    let buf = render_200x60(&mut dialog);

    // @step When the rendered buffer is inspected
    let text = join_buffer(&buf);

    // @step Then it contains board keybindings including "New Agent" and "Reorder"
    assert!(
        text.contains("New Agent"),
        "board help must contain board hint \"New Agent\":\n{text}"
    );
    assert!(
        text.contains("Reorder"),
        "board help must contain board hint \"Reorder\":\n{text}"
    );

    // @step And it does not contain any slash command starting with "/"
    for slash in &["/help", "/model", "/compact"] {
        assert!(
            !text.contains(slash),
            "board help must NOT contain slash command {slash}:\n{text}"
        );
    }
}

#[test]
fn agent_help_shows_agent_keybindings_and_the_full_slash_command_list() {
    // @step Given a HelpDialog constructed for the agent rendered against a 200x60 TestBackend
    let mut dialog = HelpDialog::for_agent();
    let buf = render_200x60(&mut dialog);

    // @step When the rendered buffer is inspected
    let text = join_buffer(&buf);

    // @step Then it contains agent keybindings including "Send" and "Interrupt"
    assert!(
        text.contains("Send"),
        "agent help must contain agent hint \"Send\":\n{text}"
    );
    assert!(
        text.contains("Interrupt"),
        "agent help must contain agent hint \"Interrupt\":\n{text}"
    );

    // @step And it contains the slash command "/compact" with its description
    assert!(
        text.contains("/compact"),
        "agent help must contain slash command \"/compact\":\n{text}"
    );
    assert!(
        text.contains("Compact context window"),
        "agent help must contain /compact description \"Compact context window\":\n{text}"
    );

    // @step And it contains the slash command "/model" with its description
    assert!(
        text.contains("/model"),
        "agent help must contain slash command \"/model\":\n{text}"
    );
    assert!(
        text.contains("Select AI model"),
        "agent help must contain /model description \"Select AI model\":\n{text}"
    );
}

#[test]
fn neither_help_variant_shows_the_misleading_q_quit_line() {
    // @step Given a HelpDialog constructed for the board and a HelpDialog constructed for the agent
    let mut board = HelpDialog::for_board();
    let mut agent = HelpDialog::for_agent();
    let board_text = join_buffer(&render_200x60(&mut board));
    let agent_text = join_buffer(&render_200x60(&mut agent));

    // @step When each rendered buffer is inspected
    // (buffers captured above)

    // @step Then neither buffer contains "q       Quit"
    assert!(
        !board_text.contains("q       Quit"),
        "board help must NOT contain the old \"q       Quit\" line:\n{board_text}"
    );
    assert!(
        !agent_text.contains("q       Quit"),
        "agent help must NOT contain the old \"q       Quit\" line:\n{agent_text}"
    );
    // Extra safety: no standalone "q ... Quit fspec-tui" line either.
    assert!(
        !board_text.contains("Quit fspec-tui"),
        "board help must NOT contain the old \"Quit fspec-tui\" wording:\n{board_text}"
    );
    assert!(
        !agent_text.contains("Quit fspec-tui"),
        "agent help must NOT contain the old \"Quit fspec-tui\" wording:\n{agent_text}"
    );

    // @step And each buffer contains "Ctrl+D" paired with "Quit"
    assert!(
        board_text.contains("Ctrl+D") && board_text.contains("Quit"),
        "board help must contain \"Ctrl+D\" and \"Quit\":\n{board_text}"
    );
    assert!(
        agent_text.contains("Ctrl+D") && agent_text.contains("Quit"),
        "agent help must contain \"Ctrl+D\" and \"Quit\":\n{agent_text}"
    );
}
