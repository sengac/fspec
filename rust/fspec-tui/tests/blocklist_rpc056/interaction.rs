//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature

use super::*;
use codelet_test_helpers::TestResult;

#[test]
fn down_advances_focused_row() -> TestResult {
    // @step Given a BlocklistView seeded with three rules with selected_index 0
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule("a", "a", "block", "system"),
        rule("b", "b", "block", "system"),
        rule("c", "c", "block", "system"),
    ]);
    assert_eq!(view.selected_index, 0);

    // @step When the user presses Down
    let _ = view.handle_key(key(KeyCode::Down));
    // @step Then selected_index equals 1
    assert_eq!(view.selected_index, 1);

    // @step When the user presses Down
    let _ = view.handle_key(key(KeyCode::Down));
    // @step Then selected_index equals 2
    assert_eq!(view.selected_index, 2);

    // Boundary: pressing Down again should clamp at 2.
    let _ = view.handle_key(key(KeyCode::Down));
    assert_eq!(view.selected_index, 2);
    Ok(())
}

#[test]
fn up_retreats_focused_row_clamped() -> TestResult {
    // @step Given a BlocklistView seeded with three rules with selected_index 1
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule("a", "a", "block", "system"),
        rule("b", "b", "block", "system"),
        rule("c", "c", "block", "system"),
    ]);
    view.selected_index = 1;

    // @step When the user presses Up
    let _ = view.handle_key(key(KeyCode::Up));
    // @step Then selected_index equals 0
    assert_eq!(view.selected_index, 0);

    // @step When the user presses Up
    let _ = view.handle_key(key(KeyCode::Up));
    // @step Then selected_index equals 0
    assert_eq!(view.selected_index, 0);
    Ok(())
}

#[test]
fn space_emits_toggle_action_for_focused_rule() -> TestResult {
    // @step Given a BlocklistView seeded with rule "git-checkout-block" focused (selected_index 0)
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule(
        "git-checkout-block",
        "^git\\s+checkout\\b",
        "block",
        "system",
    )]);
    // The view only emits the action; the App asks the daemon (BUG-601).

    // @step When the user presses Space
    let out = view.handle_key(key(KeyCode::Char(' ')));

    // @step Then a Toggle Rule action for "git-checkout-block" is emitted
    match out {
        BlocklistEvent::Emit(action) => {
            let Action::ToggleBlocklistRule(id) = *action else {
                return Err(format!("expected ToggleBlocklistRule action; got {action:?}").into());
            };
            assert_eq!(id, "git-checkout-block");
        }
        _ => return Err(format!("expected ToggleBlocklistRule action; got {out:?}").into()),
    }

    // @step When the user presses Space again
    let out = view.handle_key(key(KeyCode::Char(' ')));
    // @step Then a Toggle Rule action for "git-checkout-block" is emitted again
    assert!(matches!(
        out,
        BlocklistEvent::Emit(ref a) if matches!(a.as_ref(), Action::ToggleBlocklistRule(_))
    ));
    Ok(())
}

#[test]
fn enter_emits_toggle_action_for_focused_rule() -> TestResult {
    // @step Given a BlocklistView seeded with rule "cat-block" focused
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("cat-block", "^cat\\s+", "block", "project")]);

    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then a Toggle Rule action for "cat-block" is emitted
    match out {
        BlocklistEvent::Emit(action) => {
            let Action::ToggleBlocklistRule(id) = *action else {
                return Err(format!("expected ToggleBlocklistRule action; got {action:?}").into());
            };
            assert_eq!(id, "cat-block");
        }
        _ => return Err(format!("expected ToggleBlocklistRule action; got {out:?}").into()),
    }
    Ok(())
}
