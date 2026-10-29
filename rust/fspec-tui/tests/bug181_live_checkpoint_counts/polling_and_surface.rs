#[tokio::test]
async fn opening_and_closing_the_checkpoints_view_re_polls_the_checkpoint_counts() {
    // @step Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 2, auto: 1 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    // Bootstrap re-reads the same (2,1) value, so re-seed the stale value
    // directly to isolate the Open/Close re-poll behaviour.
    drain_bus(&mut app);
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // @step When Action::OpenCheckpointsView is dispatched and then Action::CloseCheckpointsView is dispatched
    app.dispatch(Action::OpenCheckpointsView);
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 2, auto: 1 },
        Duration::from_millis(1000),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 1 },
        "opening the view must re-poll the counts"
    );
    // Re-stale the store, then close the view.
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });
    app.dispatch(Action::CloseCheckpointsView);
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 2, auto: 1 },
        Duration::from_millis(1000),
    )
    .await;

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } so the header repaints with the live counts
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 1 },
        "closing the view must re-poll the counts (degraded-transport mitigation)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: RefreshCheckpointCounts re-reads the counts and updates the
// BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_checkpoint_counts_re_reads_the_counts_and_updates_the_board_store() {
    // @step Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 3, auto: 1 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // @step When Action::RefreshCheckpointCounts is dispatched (the follow-up the Checkpoints view emits after a successful in-view restore or delete)
    app.dispatch(Action::RefreshCheckpointCounts);
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 3, auto: 1 },
        Duration::from_millis(1000),
    )
    .await;

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 }
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 3, auto: 1 },
        "the existing RefreshCheckpointCounts flow must keep landing fresh counts in the BoardStore"
    );
    assert!(mock.checkpoint_counts_calls() >= 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecBackend exposes a transport-agnostic git-state push
// surface (source-shape)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fspec_backend_exposes_a_transport_agnostic_git_state_push_surface() {
    // @step Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-182 lands
    let root = common::workspace_root();
    let transport_mod = codelet_test_helpers::source_composition::read_source(
        &root
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("mod.rs"),
    )
    .expect("resolve composed transport source");
    let rpc_lib = codelet_test_helpers::source_composition::read_source(
        &root.join("rpc").join("src").join("lib.rs"),
    )
    .expect("resolve composed RPC source");
    let core_watcher =
        common::read_to_string_or_panic(&root.join("core").join("src").join("git_state.rs"));
    let bootstrap = common::read_to_string_or_panic(
        &root
            .join("fspec-tui")
            .join("src")
            .join("app")
            .join("bootstrap.rs"),
    );

    // @step When a developer scans the source trees
    // @step Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method git_state_changed_rx returning broadcast::Receiver<GitState>
    assert!(
        common::strip_rust_comments(&transport_mod)
            .contains("fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState>"),
        "FspecBackend must declare the git-state push receiver"
    );

    // @step And SharedFspecService in rpc/src/lib.rs exposes git_state_changed_rx and git_state_snapshot
    assert!(
        rpc_lib.contains("pub fn git_state_changed_rx"),
        "SharedFspecService must expose git_state_changed_rx()"
    );
    assert!(
        rpc_lib.contains("pub fn git_state_snapshot"),
        "SharedFspecService must expose git_state_snapshot()"
    );

    // @step And the GitStateWatcher type exists in codelet-core and app/bootstrap.rs subscribes to git_state_changed_rx
    assert!(
        core_watcher.contains("pub struct GitStateWatcher"),
        "codelet-core must define the GitStateWatcher type"
    );
    assert!(
        bootstrap.contains("git_state_changed_rx"),
        "app/bootstrap.rs must subscribe to git_state_changed_rx"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (extra guard): delete path end-to-end through the embedded
// transport (BUG-181 example 3 — in-TUI delete is a regression guard).
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedded_transport_delivers_a_decremented_frame_after_a_checkpoint_delete() {
    // @step Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with <initial_manual> manual and <initial_auto> auto checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: <initial_manual>, auto: <initial_auto> }
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");
    let service = service_for(repo);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 1, auto: 1 },
        Duration::from_secs(5),
    )
    .await;

    // @step When a manual ghost-checkpoint ref is <mutation> the repo while the App is running
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").expect("delete");

    // @step Then the App's git-state subscriber folds the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: <final_manual>, auto: <final_auto> } without any TUI restart
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 0, auto: 1 },
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 1 }
    );
}
