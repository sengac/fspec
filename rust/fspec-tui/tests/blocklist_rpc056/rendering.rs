//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature

use super::*;

#[test]
fn blocklist_view_renders_two_rules_with_source_tags() {
    // @step Given a BlocklistView seeded with rules [git-checkout-block(system, block), cat-block(project, block)]
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "git-checkout-block"
    assert!(
        text.contains("git-checkout-block"),
        "missing git-checkout-block: {text}"
    );
    // @step And the rendered text contains "cat-block"
    assert!(text.contains("cat-block"), "missing cat-block: {text}");
    // @step And the rendered text contains "system"
    assert!(text.contains("system"), "missing system source tag");
    // @step And the rendered text contains "project"
    assert!(text.contains("project"), "missing project source tag");
}

#[test]
fn empty_blocklist_renders_placeholder() {
    // @step Given a BlocklistView seeded with an empty rule list
    let mut view = BlocklistView::new();
    view.set_rules(Vec::new());

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "No blocklist rules configured"
    assert!(
        text.contains("No blocklist rules configured"),
        "missing placeholder text"
    );
}

#[test]
fn disabled_rule_paints_dimmed_glyph_and_suffix() {
    // @step Given a BlocklistView seeded with rules [git-checkout-block(system), cat-block(project)]
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);
    // @step And the focused session's blocklist_disabled set contains "git-checkout-block"
    let mut disabled: HashSet<String> = HashSet::new();
    disabled.insert("git-checkout-block".to_string());

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &disabled);

    // @step Then the rendered text contains "○ git-checkout-block"
    assert!(
        text.contains("○ git-checkout-block"),
        "expected dimmed glyph for disabled rule; got: {text}"
    );
    // @step And the rendered text contains "(disabled)"
    assert!(
        text.contains("(disabled)"),
        "expected (disabled) suffix on disabled row"
    );
    // @step And the rendered text contains "● cat-block"
    assert!(
        text.contains("● cat-block"),
        "expected enabled glyph for cat-block; got: {text}"
    );
}

#[test]
fn category_column_renders_file_path_for_path_patterns() {
    // @step Given a BlocklistView with a rule whose pattern is "/etc/passwd"
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("etc", "/etc/passwd", "block", "system")]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "file_path"
    assert!(
        text.contains("file_path"),
        "expected file_path category; got: {text}"
    );
}

#[test]
fn category_column_renders_file_path_for_tilde_patterns() {
    // @step Given a BlocklistView with a rule whose pattern is "~/.aws/.*"
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("aws", "~/.aws/.*", "block", "system")]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "file_path"
    assert!(text.contains("file_path"));
}

#[test]
fn category_column_renders_bash_for_command_patterns() {
    // @step Given a BlocklistView with a rule whose pattern is "^cat\\s+"
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("cat", "^cat\\s+", "block", "system")]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "bash"
    assert!(text.contains("bash"), "expected bash category; got: {text}");
    // @step And the rendered text does NOT contain "file_path"
    assert!(
        !text.contains("file_path"),
        "did not expect file_path category"
    );
}

#[test]
fn derive_category_is_deterministic() {
    // @step Given the derive_category helper function
    // @step Then derive_category("^cat\\s+") returns "bash"
    assert_eq!(derive_category("^cat\\s+"), "bash");
    // @step And derive_category("/etc/passwd") returns "file_path"
    assert_eq!(derive_category("/etc/passwd"), "file_path");
    // @step And derive_category("~/.aws/.*") returns "file_path"
    assert_eq!(derive_category("~/.aws/.*"), "file_path");
    // @step And derive_category("./scripts/deploy.sh") returns "file_path"
    assert_eq!(derive_category("./scripts/deploy.sh"), "file_path");
}
