//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature

use super::*;

#[test]
fn down_advances_focused_row() {
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
}

#[test]
fn up_retreats_focused_row_clamped() {
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
}

#[test]
fn space_emits_toggle_action_for_focused_rule() {
    // @step Given a BlocklistView seeded with rule "git-checkout-block" focused (selected_index 0)
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule(
        "git-checkout-block",
        "^git\\s+checkout\\b",
        "block",
        "system",
    )]);
    // @step And the session-disabled set is empty for the focused session
    // (per Action design — toggling emits the Action, the App folds it
    // into the store; the view is stateless w.r.t. the disabled set)

    // @step When the user presses Space
    let out = view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the focused session's blocklist_disabled set contains "git-checkout-block"
    // Verified via the emitted Action — the App's dispatch_blocklist
    // handler is the side that mutates AgentViewStore.
    match out {
        BlocklistEvent::Emit(action) => {
            let Action::ToggleBlocklistRule(id) = *action else {
                panic!("expected ToggleBlocklistRule action; got {action:?}");
            };
            assert_eq!(id, "git-checkout-block");
        }
        _ => panic!("expected ToggleBlocklistRule action; got {out:?}"),
    }

    // @step When the user presses Space again
    let out = view.handle_key(key(KeyCode::Char(' ')));
    // @step Then the focused session's blocklist_disabled set no longer contains "git-checkout-block"
    // The view emits the same Action again — the App's handler
    // toggles (insert ↔ remove) so the second press removes the id.
    assert!(matches!(
        out,
        BlocklistEvent::Emit(ref a) if matches!(a.as_ref(), Action::ToggleBlocklistRule(_))
    ));
}

#[test]
fn enter_emits_toggle_action_for_focused_rule() {
    // @step Given a BlocklistView seeded with rule "cat-block" focused
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("cat-block", "^cat\\s+", "block", "project")]);

    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then the focused session's blocklist_disabled set contains "cat-block"
    match out {
        BlocklistEvent::Emit(action) => {
            let Action::ToggleBlocklistRule(id) = *action else {
                panic!("expected ToggleBlocklistRule action; got {action:?}");
            };
            assert_eq!(id, "cat-block");
        }
        _ => panic!("expected ToggleBlocklistRule action; got {out:?}"),
    }
}
