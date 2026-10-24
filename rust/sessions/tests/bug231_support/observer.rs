use super::{Fixture, Result};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_unit_subscription::WorkUnitSubscription;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

struct Released(Arc<AtomicBool>);

impl Drop for Released {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

pub async fn verify(fixture: &Fixture) -> Result {
    let (sender, receiver) = tokio::sync::broadcast::channel(1);
    sender.send(Vec::new())?;
    sender.send(Vec::new())?;
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let manager = fixture.manager.clone();
    let released = Arc::new(AtomicBool::new(false));
    let resource = Released(released.clone());
    fixture
        .manager
        .set_session_focus(&fixture.first, Some("AUTH-002"), "setup")?;
    fixture.shell("AUTH-002", "validating")?;
    fixture.shell("AUTH-002", "done")?;
    let subscription = WorkUnitSubscription::from_receiver(
        fixture.project.path().to_owned(),
        receiver,
        move |project| {
            let _resource = &resource;
            count.fetch_add(1, Ordering::SeqCst);
            let result = manager.reconcile_work_units(project);
            assert!(result.is_ok(), "{result:?}");
        },
    );
    fixture.wait_focus(&fixture.first, None).await?;
    tokio::time::timeout(Duration::from_secs(5), async {
        while calls.load(Ordering::SeqCst) < 3 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    drop(subscription);
    tokio::time::timeout(Duration::from_secs(1), async {
        while !released.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    let previous = calls.load(Ordering::SeqCst);
    fixture
        .manager
        .set_session_focus(&fixture.first, Some("AUTH-002"), "followup")?;
    let _ = sender.send(Vec::new());
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(calls.load(Ordering::SeqCst), previous);
    assert!(
        fixture
            .manager
            .get_work_unit_focus(&fixture.first)?
            .revision
            > before.revision
    );
    Ok(())
}
