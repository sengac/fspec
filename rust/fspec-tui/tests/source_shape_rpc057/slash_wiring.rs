/// Scenario: /merge-worktree slash command wiring lives in dispatch_merge_worktree.rs
#[test]
fn slash_merge_worktree_wiring_declares_required_app_methods() {
    // @step Given the file rust/fspec-tui/src/app/dispatch_merge_worktree.rs exists
    let path = workspace_root().join("rust/fspec-tui/src/app/dispatch_merge_worktree.rs");
    assert!(path.is_file(), "merge-worktree dispatch source must exist");
    let source = fs::read_to_string(&path).expect("read merge-worktree dispatch source");
    let syntax = syn::parse_file(&source).expect("parse merge-worktree dispatch source");
    let method_names = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(implementation) if implementation.trait_.is_none() => {
                match implementation.self_ty.as_ref() {
                    syn::Type::Path(self_type) if self_type.path.is_ident("App") => {
                        Some(implementation)
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .flat_map(|implementation| implementation.items.iter())
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    // @step Then it declares a method named "handle_slash_merge_worktree"
    assert!(
        method_names
            .iter()
            .any(|name| name == "handle_slash_merge_worktree"),
        "impl App must declare handle_slash_merge_worktree"
    );

    // @step And it declares a method named "handle_inspect_changes_loaded"
    assert!(
        method_names
            .iter()
            .any(|name| name == "handle_inspect_changes_loaded"),
        "impl App must declare handle_inspect_changes_loaded"
    );

    // @step And it declares a method named "handle_merge_confirmed"
    assert!(
        method_names
            .iter()
            .any(|name| name == "handle_merge_confirmed"),
        "impl App must declare handle_merge_confirmed"
    );

    // @step And it declares a method named "handle_discard_confirmed"
    assert!(
        method_names
            .iter()
            .any(|name| name == "handle_discard_confirmed"),
        "impl App must declare handle_discard_confirmed"
    );

    // @step And it declares a method named "handle_cancel_merge_dialog"
    assert!(
        method_names
            .iter()
            .any(|name| name == "handle_cancel_merge_dialog"),
        "impl App must declare handle_cancel_merge_dialog"
    );

    // @step And it declares a method named "try_dispatch_merge_worktree"
    assert!(
        method_names
            .iter()
            .any(|name| name == "try_dispatch_merge_worktree"),
        "impl App must declare try_dispatch_merge_worktree"
    );
}
