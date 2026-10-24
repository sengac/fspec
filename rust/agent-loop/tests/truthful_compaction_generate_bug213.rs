//! Feature: spec/features/truthful-generated-compaction-activity.feature

#[path = "cmpct056/support.rs"]
mod support;

use codelet_tools::{GenerateCompactionArgs, GenerateCompactionTool};
use rig::tool::Tool;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_generated_compaction_reports_unknown_work_for_its_actual_target(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut measured_placeholders = Vec::new();
    // @step Given a <target> compaction target without measured completed-work units
    for cross_target in [false, true] {
        let server = support::Server::new(200).await;
        let fixture = support::Fixture::new(&server.url).await;
        let tool: GenerateCompactionTool = fixture.tool(&fixture.caller);
        let session_id = cross_target.then(|| fixture.target.id.to_string());
        let target = if cross_target {
            &fixture.target
        } else {
            &fixture.caller
        };
        let before = support::snapshot(target).await;
        let mut events = target.subscribe_to_stream();
        // @step When GenerateCompaction reaches its real pending builder
        let task = if cross_target {
            fixture.start()
        } else {
            tokio::spawn(async move { tool.call(GenerateCompactionArgs { session_id }).await })
        };
        let reached = tokio::time::timeout(Duration::from_secs(3), server.arrived.wait()).await;
        let progress = target.get_compaction_progress();
        task.abort();
        let _ = task.await;
        reached?;
        server.release.wait().await;
        // @step Then its target activity reports the phase actually reached and unknown work counts
        let progress = progress.ok_or("GenerateCompaction activity missing")?;
        assert_eq!(progress.phase, "Compaction sub-agent working");
        if (progress.current, progress.total) != (0, 0) {
            measured_placeholders.push(format!("cross_target={cross_target}: {progress:?}"));
        }
        // @step And dropping the operation clears activity without a committed generation
        assert!(target.get_compaction_progress().is_none());
        support::assert_clean(target);
        support::assert_no_completion(&mut events);
        assert_eq!(support::snapshot(target).await, before);
        assert!(codelet_core::persistence::load_session(target.id)?
            .compaction_commit
            .is_none());
    }
    assert!(
        measured_placeholders.is_empty(),
        "{}",
        measured_placeholders.join("; ")
    );
    Ok(())
}
