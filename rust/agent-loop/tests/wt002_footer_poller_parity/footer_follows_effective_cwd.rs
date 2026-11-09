//! Feature: spec/features/fspec-binary-footer-poller.feature
use super::*;

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_footer_follows_effective_cwd_when_bash_moves_the_session_to_another_directory() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new session has been created whose footer CWD has been seeded to the project root
    let repo = fresh_git_repo();
    let project = repo.path().to_str().expect("repo path is utf8");
    let sub = repo.path().join("src");
    std::fs::create_dir_all(&sub).expect("create subdirectory");
    let sub_str = sub.to_str().expect("subdir path is utf8").to_string();
    let sid = Uuid::new_v4();
    manager
        .create_session_with_id(
            &sid.to_string(),
            "anthropic/claude-opus-4-5",
            project,
            "wt002",
        )
        .await
        .expect("session creation under FspecAgentHooks must succeed");
    let initial = next_footer_chunk(&mut chunks_rx, &SessionId::new(sid.to_string()), |c| {
        matches!(c, StreamChunk::FooterStateUpdate { .. })
    })
    .await;
    match initial {
        StreamChunk::FooterStateUpdate { cwd, .. } => {
            assert_eq!(
                cwd, project,
                "first tick must seed the CWD to the project root"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }

    // @step When the session's footer CWD registry is updated to a subdirectory
    codelet_tools::footer_cwd::update_footer_cwd(sid, sub_str.clone());

    // @step And the footer poller ticks again
    // @step Then the emitted footer state carries the subdirectory as the CWD
    let updated = next_footer_chunk(
        &mut chunks_rx,
        &SessionId::new(sid.to_string()),
        |c| match c {
            StreamChunk::FooterStateUpdate { cwd, .. } => cwd == &sub_str,
            _ => false,
        },
    )
    .await;
    match updated {
        StreamChunk::FooterStateUpdate { cwd, .. } => {
            assert_eq!(
                cwd, sub_str,
                "the second emission must carry the subdirectory"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }
}
