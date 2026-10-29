//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::*;

#[tokio::test]
#[serial_test::serial]
async fn unicode_details_wrap_scroll_and_retain_focus_on_resize() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given long Unicode details with a selectable linked card near the end
    let mut fixture = Canonical::new(false).await?;
    open_details(&mut fixture.app).await?;
    // @step When the user scrolls by arrows pages endpoints and wheel then resizes narrowly
    let _ = buffer(&mut fixture.app, 42, 18);
    press(&mut fixture.app, KeyCode::Home);
    let first = buffer_text(&buffer(&mut fixture.app, 42, 18));
    press(&mut fixture.app, KeyCode::Down);
    let down = buffer_text(&buffer(&mut fixture.app, 42, 18));
    assert_ne!(down, first, "Down scrolls one wrapped row");
    press(&mut fixture.app, KeyCode::Up);
    assert_eq!(buffer_text(&buffer(&mut fixture.app, 42, 18)), first);
    press(&mut fixture.app, KeyCode::PageDown);
    assert_ne!(buffer_text(&buffer(&mut fixture.app, 42, 18)), down);
    press(&mut fixture.app, KeyCode::PageUp);
    assert_eq!(buffer_text(&buffer(&mut fixture.app, 42, 18)), first);
    wheel(&mut fixture.app, MouseEventKind::ScrollDown);
    assert_ne!(buffer_text(&buffer(&mut fixture.app, 42, 18)), first);
    wheel(&mut fixture.app, MouseEventKind::ScrollUp);
    // @step Then the first and final wrapped lines remain reachable without clipping
    let all = harvest(&mut fixture.app, 42, 18);
    assert!(all.contains("FIRST-CANONICAL-LINE"));
    assert!(all.contains("FINAL-CANONICAL-LINE"));
    press(&mut fixture.app, KeyCode::End);
    let last = buffer_text(&buffer(&mut fixture.app, 42, 18));
    press(&mut fixture.app, KeyCode::PageDown);
    assert_eq!(buffer_text(&buffer(&mut fixture.app, 42, 18)), last);
    // @step And Tab reveals the selected link and resize preserves its activation target
    focus_label_at(&mut fixture.app, "AUTH-002", 42, 18);
    let _ = buffer(&mut fixture.app, 26, 12);
    let _ = buffer(&mut fixture.app, 100, 30);
    press(&mut fixture.app, KeyCode::Enter);
    settle(&mut fixture.app).await?;
    assert!(text(&mut fixture.app).contains("B-CANONICAL-DETAIL"));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn linked_card_cycles_support_back_history_and_missing_targets_retain_context() -> TestResult
{
    let _isolation = Isolation::new()?;
    // @step Given details for linked cards A and B and a missing linked card
    let mut fixture = Canonical::new(false).await?;
    open_details(&mut fixture.app).await?;
    // @step When Tab and Enter follow A to B to A and Escape unwinds that history
    focus_label(&mut fixture.app, "AUTH-002");
    let parent = text(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Enter);
    settle(&mut fixture.app).await?;
    assert!(text(&mut fixture.app).contains("B-CANONICAL-DETAIL"));
    focus_label(&mut fixture.app, "AUTH-001");
    let child = text(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Enter);
    settle(&mut fixture.app).await?;
    assert!(harvest(&mut fixture.app, 120, 36).contains("FIRST-CANONICAL-LINE"));
    // @step Then each parent restores its scroll and focused link before returning to the chooser
    press(&mut fixture.app, KeyCode::Esc);
    assert_eq!(text(&mut fixture.app), child);
    press(&mut fixture.app, KeyCode::Esc);
    assert_eq!(text(&mut fixture.app), parent);
    // @step And opening the missing link shows an error while the current details remain usable
    focus_label(&mut fixture.app, "AUTH-404");
    press(&mut fixture.app, KeyCode::Enter);
    settle(&mut fixture.app).await?;
    let failed = text(&mut fixture.app);
    assert!(failed.contains("AUTH-404"));
    assert!(
        failed.to_lowercase().contains("not found") || failed.to_lowercase().contains("missing")
    );
    focus_label(&mut fixture.app, "AUTH-002");
    press(&mut fixture.app, KeyCode::Esc);
    assert_chooser(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Esc);
    assert!(fixture.app.compositor().is_empty());
    assert_eq!(fixture.app.active_view(), ViewMode::Board);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn details_consume_modal_shortcuts_and_paste_without_changing_underlying_state() -> TestResult
{
    let _isolation = Isolation::new()?;
    for mux in [false, true] {
        // @step Given complete details are open above a board or mux with an unrelated agent
        let mut fixture = Canonical::new(false).await?;
        fixture.app.agent_view_store_mut().append_session(
            codelet_fspec_tui::store::SessionContext::new(SessionId::new("unrelated")),
        );
        fixture.app.agent_view_store_mut().set_work_unit_context(
            SessionId::new("unrelated"),
            WorkUnitContext {
                id: "AUTH-002".into(),
                title: "Unrelated".into(),
                status: "backlog".into(),
            },
        );
        if mux {
            fixture.app.navigator_mut().mux.enable_default();
            fixture.app.navigator_mut().active_view = ViewMode::Mux;
            assert_eq!(fixture.app.navigator().mux.focus(), 0);
        }
        let origin = fixture.app.active_view();
        let layout = fixture.app.navigator().mux.config().clone();
        open_details(&mut fixture.app).await?;
        let layers = fixture.app.compositor().layer_ids();
        let view = fixture.app.active_view();
        let focus = fixture.app.navigator().mux.focus();
        // @step When the user types shortcut keys modified arrows and a multiline paste
        for code in [
            KeyCode::Char('?'),
            KeyCode::Char('q'),
            KeyCode::Char('m'),
            KeyCode::Char('p'),
            KeyCode::Char('a'),
        ] {
            assert!(press(&mut fixture.app, code));
        }
        for code in [KeyCode::Left, KeyCode::Right] {
            assert!(event(
                &mut fixture.app,
                Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
            ));
        }
        assert!(fixture.app.handle_paste("modal\npaste 🦀").is_consumed());
        flush(&mut fixture.app);
        // @step Then the details stay open and board selection session input and assignments remain unchanged
        assert_eq!(fixture.app.compositor().layer_ids(), layers);
        assert_eq!(fixture.app.active_view(), view);
        assert_eq!(fixture.app.navigator().mux.focus(), focus);
        assert_eq!(
            fixture.app.current_session(),
            Some(SessionId::new("unrelated"))
        );
        assert_eq!(
            fixture
                .app
                .board_store()
                .selected_work_unit()
                .map(|card| card.id.as_str()),
            Some("AUTH-001")
        );
        assert!(fixture.app.navigator().agent.input.value().is_empty());
        assert_eq!(
            fixture
                .app
                .agent_view_store()
                .work_unit_context_for(&SessionId::new("unrelated"))
                .map(|context| context.id.as_str()),
            Some("AUTH-002")
        );
        assert!(!fixture.app.should_quit());
        press(&mut fixture.app, KeyCode::Esc);
        assert_chooser(&mut fixture.app);
        assert_eq!(fixture.app.active_view(), origin);
        assert_eq!(fixture.app.navigator().mux.focus(), focus);
        assert_mux_layout_preserved(fixture.app.navigator().mux.config(), &layout);
        press(&mut fixture.app, KeyCode::Esc);
        assert!(fixture.app.compositor().is_empty());
        assert_eq!(fixture.app.active_view(), origin);
        fixture.app.dispatch(Action::OpenProviderSettingsView);
        assert_eq!(fixture.app.active_view(), ViewMode::ProviderSettings);
        fixture.app.dispatch(Action::CloseProviderSettingsView);
        settle(&mut fixture.app).await?;
        assert_eq!(fixture.app.active_view(), origin);
        assert_eq!(fixture.app.navigator().mux.focus(), focus);
        assert_mux_layout_preserved(fixture.app.navigator().mux.config(), &layout);
        assert_eq!(
            fixture.app.current_session(),
            Some(SessionId::new("unrelated"))
        );
        assert_eq!(
            fixture
                .app
                .board_store()
                .selected_work_unit()
                .map(|card| card.id.as_str()),
            Some("AUTH-001")
        );
        assert_eq!(
            fixture
                .app
                .agent_view_store()
                .work_unit_context_for(&SessionId::new("unrelated"))
                .map(|context| context.id.as_str()),
            Some("AUTH-002")
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn resized_details_use_current_mouse_geometry_and_reveal_keyboard_focus() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given details with linked cards below the initial viewport
    let mut fixture = Canonical::new(false).await?;
    open_details(&mut fixture.app).await?;
    // @step When Tab reveals a link and the terminal resizes before a mouse click
    focus_label(&mut fixture.app, "AUTH-002");
    assert!(selected_position(&buffer(&mut fixture.app, 120, 36), "AUTH-002").is_some());
    let latest = buffer(&mut fixture.app, 70, 20);
    let current =
        selected_position(&latest, "AUTH-002").ok_or("resize must reveal the focused link")?;
    // @step Then only the latest rendered link hitbox activates its visible target
    let layers = fixture.app.compositor().layer_ids();
    // @step And an old or offscreen hitbox cannot activate a stale link
    press(&mut fixture.app, KeyCode::Home);
    let top = buffer(&mut fixture.app, 70, 20);
    assert!(
        top.area.contains((current.0, current.1).into()),
        "stale click must remain in bounds"
    );
    assert!(
        selected_position(&top, "AUTH-002").is_none(),
        "fixture must scroll the link offscreen"
    );
    event(
        &mut fixture.app,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: current.0,
            row: current.1,
            modifiers: KeyModifiers::NONE,
        }),
    );
    settle(&mut fixture.app).await?;
    assert_eq!(fixture.app.compositor().layer_ids(), layers);
    assert_eq!(
        buffer_text(&buffer(&mut fixture.app, 70, 20)),
        buffer_text(&top)
    );
    focus_label_at(&mut fixture.app, "AUTH-002", 70, 20);
    let latest = buffer(&mut fixture.app, 70, 20);
    let current =
        selected_position(&latest, "AUTH-002").ok_or("focused link must remain visible")?;
    event(
        &mut fixture.app,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: current.0,
            row: current.1,
            modifiers: KeyModifiers::NONE,
        }),
    );
    settle(&mut fixture.app).await?;
    assert!(text(&mut fixture.app).contains("B-CANONICAL-DETAIL"));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn footer_and_help_describe_card_choices_and_details_controls() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given the board and its help are rendered
    let mut fixture = Canonical::new(false).await?;
    let board = buffer_text(&buffer(&mut fixture.app, 180, 36));
    assert!(
        board.to_lowercase().contains("choos"),
        "board footer must advertise the changed Enter action"
    );
    press(&mut fixture.app, KeyCode::Char('?'));
    let help = harvest(&mut fixture.app, 120, 36).to_lowercase();
    assert!(help.contains("details") && help.contains("agent"));
    press(&mut fixture.app, KeyCode::Esc);
    // @step When the user opens the card chooser and full details
    open_details(&mut fixture.app).await?;
    // @step Then hints describe the chooser and scrolling link attachment and back controls
    let rendered = text(&mut fixture.app).to_lowercase();
    for hint in ["tab", "enter", "esc", "scroll"] {
        assert!(rendered.contains(hint), "missing details control {hint}");
    }
    Ok(())
}
