// ============================================================
// Section D — ThinkingLevelDialog migration
// ============================================================

fn make_thinking_dialog(current: ThinkingLevel) -> ThinkingLevelDialog {
    ThinkingLevelDialog::new(SessionId::new("test-session"), current, four_levels())
}

/// Scenario: ThinkingLevelDialog renders with the yellow accent and inner-title body
#[test]
fn thinking_level_dialog_renders_with_yellow_accent_and_inner_title() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog)?;
    // @step Then the border cells use foreground color Color::Yellow
    assert_eq!(find_border_color(&buf), Color::Yellow);
    // @step And the body's first non-padding row contains the text "Thinking Level"
    let (x, y) = find_text(&buf, "Thinking Level").or_fail("title present")?;
    // @step And the "Thinking Level" text cells have foreground color Color::Yellow with BOLD modifier
    for i in 0..("Thinking Level".chars().count() as u16) {
        let cell = &buf[(x + i, y)];
        assert_eq!(cell.fg, Color::Yellow);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }
    Ok(())
}

/// Scenario: ThinkingLevelDialog highlights the current level with the inverse style
#[test]
fn thinking_level_dialog_highlights_the_current_level_with_inverse_style() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog)?;

    let (off_x, off_y) = find_text(&buf, "Off").or_fail("Off row present")?;
    // @step Then the "Off" row has background color Color::Yellow
    // @step And the "Off" row has foreground color Color::Black with BOLD modifier
    let cell = &buf[(off_x, off_y)];
    assert_eq!(cell.bg, Color::Yellow);
    assert_eq!(cell.fg, Color::Black);
    assert!(cell.modifier.contains(Modifier::BOLD));

    // @step And the "Off" row begins with the two-character marker "▸ "
    let marker_x = off_x - 2;
    assert_eq!(buf[(marker_x, off_y)].symbol(), "▸");
    assert_eq!(buf[(marker_x + 1, off_y)].symbol(), " ");

    // @step And the "low", "medium", and "high" rows begin with the two-character marker "  "
    for label in &["low", "medium", "high"] {
        let (lx, ly) = find_text(&buf, label).or_fail(&format!("{label} present"))?;
        let m_x = lx - 2;
        assert_eq!(
            buf[(m_x, ly)].symbol(),
            " ",
            "unselected marker[0] for {label}"
        );
        assert_eq!(
            buf[(m_x + 1, ly)].symbol(),
            " ",
            "unselected marker[1] for {label}"
        );
    }

    // @step And the description text for unselected rows carries Modifier::DIM
    let (low_x, low_y) = find_text(&buf, "low").or_fail("Low present")?;
    // Description starts after "Low - " — look a few cells to the right
    let after_label = low_x + ("Low - ".chars().count() as u16);
    let desc_cell = &buf[(after_label, low_y)];
    assert!(
        desc_cell.modifier.contains(Modifier::DIM),
        "unselected description must be DIM"
    );
    Ok(())
}

/// Scenario: ThinkingLevelDialog footer documents the four key bindings
#[test]
fn thinking_level_dialog_footer_documents_the_four_key_bindings() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut dialog)?;
    let text = buffer_text(&buf);

    // @step Then the last body row contains the substring "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"
    assert!(
        text.contains("↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"),
        "footer must include the D Set Default binding"
    );

    // @step And the footer text carries Modifier::DIM
    let (fx, fy) = find_text(&buf, "↑↓ Navigate").or_fail("footer present")?;
    assert!(buf[(fx, fy)].modifier.contains(Modifier::DIM));
    // @step And the footer text is horizontally centered
    let footer = "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close";
    let footer_width = unicode_width::UnicodeWidthStr::width(footer) as u16;
    let left_border = (0..buf.area.width)
        .find(|column| buf[(*column, fy)].symbol() == "│")
        .or_fail("footer row has a left dialog border")?;
    let right_border = (0..buf.area.width)
        .rfind(|column| buf[(*column, fy)].symbol() == "│")
        .or_fail("footer row has a right dialog border")?;
    assert!(fx > left_border);
    assert!(fx + footer_width <= right_border);
    let left_padding = fx - left_border - 1;
    let right_padding = right_border - fx - footer_width;
    assert!(left_padding.abs_diff(right_padding) <= 1);
    Ok(())
}

/// Scenario: Pressing D in ThinkingLevelDialog emits Action::SetThinkingLevelDefault and keeps the dialog open
#[test]
fn pressing_d_emits_set_thinking_level_default_and_keeps_dialog_open() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Off and currently highlighting "medium"
    let mut dialog = make_thinking_dialog(ThinkingLevel::Off);
    // Navigate to Medium (index 2)
    let _ = dialog.handle_event(&key_event(KeyCode::Down));
    let _ = dialog.handle_event(&key_event(KeyCode::Down));
    assert_eq!(
        dialog.selected_level(),
        Some(ThinkingLevel::Effort("medium".to_string()))
    );

    // @step When I send a KeyCode::Char('d') event
    let result = dialog.handle_event(&key_event(KeyCode::Char('d')));

    // @step Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::Effort("medium"))
    let action = dialog
        .take_pending_action()
        .or_fail("D must emit a pending action")?;
    match action {
        Action::SetThinkingLevelDefault(_sid, level) => {
            assert_eq!(level, ThinkingLevel::Effort("medium".to_string()));
        }
        other => return Err(format!("expected SetThinkingLevelDefault, got {other:?}").into()),
    }

    // @step And the dialog returns EventResult::Consumed without a remove-callback
    match result {
        EventResult::Consumed(None) => {}
        EventResult::Consumed(Some(_)) => {
            return Err("D must NOT supply a remove-callback — dialog stays open".into())
        }
        EventResult::Ignored(_) => return Err("D must be consumed".into()),
    }

    // @step And the dialog is still mounted on the compositor
    // (the absence of a remove-callback above is the operational proof)
    Ok(())
}

/// Scenario: Pressing uppercase D in ThinkingLevelDialog behaves identically to lowercase d
#[test]
fn pressing_uppercase_d_behaves_identically_to_lowercase_d() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with ThinkingLevel::Effort("high")
    let mut dialog = make_thinking_dialog(ThinkingLevel::Effort("high".to_string()));
    // @step When I send a KeyCode::Char('D') event
    let result = dialog.handle_event(&key_event(KeyCode::Char('D')));
    // @step Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::Effort("high"))
    let action = dialog
        .take_pending_action()
        .or_fail("uppercase D must emit pending action")?;
    match action {
        Action::SetThinkingLevelDefault(_sid, level) => {
            assert_eq!(level, ThinkingLevel::Effort("high".to_string()));
        }
        other => return Err(format!("expected SetThinkingLevelDefault, got {other:?}").into()),
    }
    // @step And the dialog is still mounted on the compositor
    match result {
        EventResult::Consumed(None) => {}
        _ => return Err("uppercase D must be Consumed(None)".into()),
    }
    Ok(())
}

/// Scenario: SetThinkingLevelDefault is wired through the backend trait stack
#[test]
fn set_thinking_level_default_is_wired_through_the_backend_trait_stack() -> TestResult {
    // @step Given the codelet_rpc_types::Action enum
    // @step Then it contains the variant SetThinkingLevelDefault(SessionId, ThinkingLevel)
    let _variant = Action::SetThinkingLevelDefault(
        SessionId::new("s"),
        ThinkingLevel::Effort("medium".to_string()),
    );

    // @step Given the SessionManagerHandle trait
    // @step Then it declares set_thinking_level_default with a default no-op implementation returning Ok(())
    //
    // This is a source-shape assertion. The trait lives in
    // rust/core/src/session_manager_handle.rs. We grep the source.
    let trait_src = codelet_test_helpers::source_composition::read_tree_or_file(
        Path::new("..")
            .join("core")
            .join("src")
            .join("session_manager_handle.rs"),
    )
    .or_else(|_| {
        codelet_test_helpers::source_composition::read_tree_or_file(
            "../core/src/session_manager_handle.rs",
        )
    })
    .or_fail("session_manager_handle.rs must exist")?;
    assert!(
        trait_src.contains("set_thinking_level_default"),
        "SessionManagerHandle must declare set_thinking_level_default"
    );

    // @step Given the FspecBackend trait
    // @step Then it declares set_thinking_level_default on both transports
    let backend_src = codelet_test_helpers::source_composition::read_source(
        &Path::new("src").join("transport").join("mod.rs"),
    )
    .or_else(|_| {
        codelet_test_helpers::source_composition::read_source(Path::new("src/transport/mod.rs"))
    })
    .or_fail("transport/mod.rs must exist")?;
    assert!(
        backend_src.contains("set_thinking_level_default"),
        "FspecBackend trait must declare set_thinking_level_default"
    );

    // RPC-027 rule [8] + architecture note [4]: App::dispatch must
    // route Action::SetThinkingLevelDefault through a handler that
    // calls backend.set_thinking_level_default. Without this wiring
    // the dialog emits an action that is silently dropped.
    // @step Then dispatch_model_thinking_dialogs.rs routes Action::SetThinkingLevelDefault to backend.set_thinking_level_default
    let dispatch_src = codelet_test_helpers::source_composition::read_tree_or_file(
        Path::new("src")
            .join("app")
            .join("dispatch_model_thinking_dialogs.rs"),
    )
    .or_else(|_| {
        codelet_test_helpers::source_composition::read_tree_or_file(
            "src/app/dispatch_model_thinking_dialogs.rs",
        )
    })
    .or_fail("dispatch_model_thinking_dialogs.rs must exist")?;
    assert!(
        dispatch_src.contains("Action::SetThinkingLevelDefault"),
        "dispatch_model_thinking_dialogs.rs must route Action::SetThinkingLevelDefault"
    );
    assert!(
        dispatch_src.contains("backend.set_thinking_level_default("),
        "dispatch_model_thinking_dialogs.rs must call backend.set_thinking_level_default"
    );
    Ok(())
}

/// MODEL-011: a model offering Off and the efforts low, medium and high.
fn four_levels() -> codelet_rpc_types::ThinkingOptions {
    codelet_rpc_types::ThinkingOptions::Levels {
        model: "test/model".to_string(),
        choices: vec![
            codelet_rpc_types::ThinkingChoice::Off,
            codelet_rpc_types::ThinkingChoice::Effort("low".to_string()),
            codelet_rpc_types::ThinkingChoice::Effort("medium".to_string()),
            codelet_rpc_types::ThinkingChoice::Effort("high".to_string()),
        ],
        takes_off: true,
    }
}
