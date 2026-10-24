use crate::{background_session::BackgroundSession, SessionManager};
use codelet_cli::session::{Session, SystemReminderType};
use codelet_core::persistence::{update_session_manifest, SessionManifest};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitFocus};
use std::sync::atomic::Ordering;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FocusError {
    #[error("session unavailable: {0}")]
    Session(String),
    #[error("cannot resolve work unit in session project: {0}")]
    Project(String),
    #[error("work unit not found in session project: {0}")]
    UnknownCard(String),
    #[error("focus state lock poisoned")]
    Lock,
    #[error("focus revision exhausted")]
    Revision,
    #[error("cannot persist focus: {0}")]
    Persistence(String),
}

pub(crate) fn resolve(
    session: &BackgroundSession,
    id: &str,
) -> Result<WorkUnitContext, FocusError> {
    codelet_core::work_units::read_snapshot(std::path::Path::new(&session.project))
        .map_err(|error| FocusError::Project(error.to_string()))?
        .into_iter()
        .find(|card| card.id == id)
        .map(|card| WorkUnitContext {
            id: card.id,
            title: card.title,
            status: card.status,
        })
        .ok_or_else(|| FocusError::UnknownCard(id.into()))
}

impl SessionManager {
    pub fn set_session_focus(
        &self,
        session_id: &SessionId,
        work_unit_id: Option<&str>,
        source: &str,
    ) -> Result<WorkUnitFocus, FocusError> {
        let session = self
            .get_session(&session_id.value)
            .map_err(FocusError::Session)?;
        let mut admission = session
            .focus_admission
            .lock()
            .map_err(|_| FocusError::Lock)?;
        let context = work_unit_id.map(|id| resolve(&session, id)).transpose()?;
        *admission = admission.checked_add(1).ok_or(FocusError::Revision)?;
        self.apply_resolved_focus(&session, context, source)
    }

    pub(crate) fn apply_resolved_focus(
        &self,
        session: &BackgroundSession,
        context: Option<WorkUnitContext>,
        source: &str,
    ) -> Result<WorkUnitFocus, FocusError> {
        let mut current = session
            .work_unit_focus
            .lock()
            .map_err(|_| FocusError::Lock)?;
        if current.context == context {
            return Ok(current.clone());
        }
        let next = WorkUnitFocus {
            revision: current
                .revision
                .checked_add(1)
                .ok_or(FocusError::Revision)?,
            context,
            source: source.into(),
        };
        update_session_manifest(session.id, |manifest| {
            manifest.work_unit_focus = next.clone();
            Ok(())
        })
        .map_err(|error| FocusError::Persistence(error.to_string()))?;
        *current = next.clone();
        let _ = self
            .work_unit_focus_tx
            .send((SessionId::from(session.id.to_string()), next.clone()));
        drop(current);
        session.schedule_focus_reminder();
        codelet_tools::broadcast_metadata_update();
        Ok(next)
    }

    pub(crate) fn restore_activity_focus(
        &self,
        session: &BackgroundSession,
        manifest: &SessionManifest,
    ) -> Result<(), String> {
        session.restore_work_unit_authorization(manifest.work_unit_authorization.clone());
        let mut focus = manifest.work_unit_focus.clone();
        let context = focus
            .context
            .as_ref()
            .map(|context| resolve(session, &context.id));
        let context = match context {
            Some(Ok(context)) if context.status != "done" => Some(context),
            Some(Err(FocusError::UnknownCard(_))) | Some(Ok(_)) | None => None,
            Some(Err(error)) => return Err(error.to_string()),
        };
        if focus.context != context {
            focus.context = context;
            focus.revision = focus
                .revision
                .checked_add(1)
                .ok_or("focus revision exhausted")?;
            focus.source = "restore".into();
            update_session_manifest(session.id, |current| {
                current.work_unit_focus = focus.clone();
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        }
        *session
            .work_unit_focus
            .lock()
            .map_err(|error| error.to_string())? = focus;
        session.schedule_focus_reminder();
        Ok(())
    }
}

impl BackgroundSession {
    pub fn activity_focus(&self) -> WorkUnitFocus {
        self.work_unit_focus
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn reconcile_focus_reminder(&self, inner: &mut Session) {
        let focus = self.activity_focus();
        if self.focus_reminder_revision.load(Ordering::Acquire) != focus.revision {
            let base = self.base_environment_for_focus();
            apply_reminder(inner, &base, &focus);
            self.focus_reminder_revision
                .store(focus.revision, Ordering::Release);
        }
    }

    fn schedule_focus_reminder(&self) {
        if let Ok(mut inner) = self.inner.try_lock() {
            self.reconcile_focus_reminder(&mut inner);
            return;
        }
        let inner = self.inner.clone();
        let focus = self.work_unit_focus.clone();
        let applied = self.focus_reminder_revision.clone();
        let base = self.base_environment_for_focus();
        self.focus_runtime.spawn(async move {
            let mut inner = inner.lock().await;
            let current = focus
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            if applied.load(Ordering::Acquire) != current.revision {
                apply_reminder(&mut inner, &base, &current);
                applied.store(current.revision, Ordering::Release);
            }
        });
    }
}

fn apply_reminder(inner: &mut Session, base: &str, focus: &WorkUnitFocus) {
    let id = focus
        .context
        .as_ref()
        .map_or("NONE", |context| context.id.as_str());
    inner.add_system_reminder(
        SystemReminderType::Environment,
        &format!("{base}\nCurrent work unit: {id}"),
    );
}
