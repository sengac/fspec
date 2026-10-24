use super::{Fixture, Result};
use codelet_tools::execution_context::WorkUnitAttribution;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub fn bind_before_deletion(
    fixture: &Fixture,
    session: &codelet_sessions::background_session::BackgroundSession,
) -> Result {
    fixture.shell("AUTH-002", "specifying")?;
    session.set_work_unit_context(
        Some("AUTH-002".into()),
        Some("bound".into()),
        Some("specifying".into()),
    );
    Ok(())
}

pub fn assert_deleted_binding(session: &codelet_sessions::background_session::BackgroundSession) {
    assert_eq!(
        session
            .get_work_unit_context()
            .and_then(|context| context.status)
            .as_deref(),
        Some("specifying")
    );
}

pub async fn verify_callback_lock(fixture: &Fixture) -> Result {
    let session = fixture.manager.get_session(&fixture.first.value)?;
    let blocked = Arc::new(AtomicBool::new(false));
    let observed = blocked.clone();
    codelet_tools::set_session_list_provider(Some(Arc::new(move || {
        let (sender, receiver) = std::sync::mpsc::channel();
        let session = session.clone();
        std::thread::spawn(move || {
            let _ = sender.send(session.get_work_unit_context());
        });
        if receiver.recv_timeout(Duration::from_secs(1)).is_err() {
            observed.store(true, Ordering::SeqCst);
        }
        Vec::new()
    })));
    let result = fixture.status("AUTH-001", "implementing").await;
    codelet_tools::set_session_list_provider(None);
    result?;
    assert!(
        !blocked.load(Ordering::SeqCst),
        "metadata callback cannot read authorization while reconciliation holds its lock"
    );
    Ok(())
}

pub fn verify_metadata_race(fixture: &Fixture) -> Result {
    fixture
        .manager
        .set_session_focus(&fixture.first, Some("AUTH-002"), "setup")?;
    let receipt = fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Absent)?;
    fixture.shell("AUTH-002", "validating")?;
    fixture.observe()?;
    fixture.shell("AUTH-003", "validating")?;
    fixture
        .manager
        .reconcile_status_success(&receipt, "AUTH-003")?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-003"));
    Ok(())
}
