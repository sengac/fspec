use crate::{
    activity_focus::{resolve, FocusError},
    SessionManager,
};
use codelet_core::{
    persistence::update_session_manifest,
    work_units::{read_snapshot, WorkUnitsWatcher},
};
use codelet_rpc_types::{SessionId, WorkUnitContext};
use codelet_tools::execution_context::{InvocationReceipt, WorkUnitAttribution};
use std::{path::Path, sync::Arc};

impl SessionManager {
    pub fn admit_tool_invocation(
        &self,
        id: &SessionId,
        attribution: WorkUnitAttribution,
    ) -> Result<InvocationReceipt, FocusError> {
        let session = self.get_session(&id.value).map_err(FocusError::Session)?;
        let mut admission = session
            .focus_admission
            .lock()
            .map_err(|_| FocusError::Lock)?;
        let context = match &attribution {
            WorkUnitAttribution::Card(id) => Some(Some(resolve(&session, id)?)),
            WorkUnitAttribution::None => Some(None),
            WorkUnitAttribution::Absent => None,
        };
        let next = admission.checked_add(1).ok_or(FocusError::Revision)?;
        if let Some(context) = context {
            self.apply_resolved_focus(&session, context, "tool")?;
        }
        *admission = next;
        Ok(InvocationReceipt {
            session_id: session.id,
            project: Path::new(&session.project).to_path_buf(),
            attribution,
            admission: next,
            focus_revision: session.activity_focus().revision,
        })
    }

    pub fn reconcile_status_success(
        &self,
        receipt: &InvocationReceipt,
        target: &str,
    ) -> Result<(), FocusError> {
        let session = self
            .get_session(&receipt.session_id.to_string())
            .map_err(FocusError::Session)?;
        if !same_project(&receipt.project, Path::new(&session.project)) {
            return Err(FocusError::Project("invocation project mismatch".into()));
        }
        {
            let admission = session
                .focus_admission
                .lock()
                .map_err(|_| FocusError::Lock)?;
            if *admission == receipt.admission
                && session.activity_focus().revision >= receipt.focus_revision
                && receipt.attribution == WorkUnitAttribution::Absent
            {
                let context = resolve(&session, target)?;
                self.apply_resolved_focus(
                    &session,
                    (context.status != "done").then_some(context),
                    "status",
                )?;
            }
        }
        self.reconcile_work_units(&receipt.project)
    }

    pub fn reconcile_work_units(&self, project: &Path) -> Result<(), FocusError> {
        let sessions: Vec<_> = self
            .sessions
            .read()
            .map_err(|_| FocusError::Lock)?
            .values()
            .cloned()
            .collect();
        for session in sessions {
            if !same_project(project, Path::new(&session.project)) {
                continue;
            }
            let _admission = session
                .focus_admission
                .lock()
                .map_err(|_| FocusError::Lock)?;
            let cards =
                read_snapshot(project).map_err(|error| FocusError::Project(error.to_string()))?;
            session.refresh_authorization(&cards)?;
            let focus = session.activity_focus();
            let Some(current) = focus.context else {
                continue;
            };
            let context =
                cards
                    .iter()
                    .find(|card| card.id == current.id)
                    .map(|card| WorkUnitContext {
                        id: card.id.clone(),
                        title: card.title.clone(),
                        status: card.status.clone(),
                    });
            let context = context.filter(|card| card.status != "done" || current.status == "done");
            self.apply_resolved_focus(&session, context, "watcher")?;
        }
        Ok(())
    }

    pub fn watch_work_units(
        self: &Arc<Self>,
        watcher: &WorkUnitsWatcher,
    ) -> codelet_core::work_unit_subscription::WorkUnitSubscription {
        let manager = Arc::downgrade(self);
        codelet_core::work_unit_subscription::WorkUnitSubscription::new(watcher, move |project| {
            if let Some(manager) = manager.upgrade() {
                if let Err(error) = manager.reconcile_work_units(project) {
                    tracing::warn!(%error, "work unit focus reconciliation failed");
                }
            }
        })
    }
}

pub(crate) fn same_project(first: &Path, second: &Path) -> bool {
    match (first.canonicalize(), second.canonicalize()) {
        (Ok(first), Ok(second)) => first == second,
        _ => first == second,
    }
}

impl crate::background_session::BackgroundSession {
    fn refresh_authorization(
        &self,
        cards: &[codelet_rpc_types::WorkUnitInfo],
    ) -> Result<(), FocusError> {
        let mut authorization = self
            .work_unit_context
            .write()
            .map_err(|_| FocusError::Lock)?;
        let Some(current) = authorization.as_ref() else {
            return Ok(());
        };
        let Some(card) = cards
            .iter()
            .find(|card| Some(&card.id) == current.id.as_ref())
        else {
            return Ok(());
        };
        if current.status.as_ref() == Some(&card.status)
            && current.title.as_ref() == Some(&card.title)
        {
            return Ok(());
        }
        let context = WorkUnitContext {
            id: card.id.clone(),
            title: card.title.clone(),
            status: card.status.clone(),
        };
        update_session_manifest(self.id, |manifest| {
            manifest.work_unit_authorization = Some(context);
            Ok(())
        })
        .map_err(|error| FocusError::Persistence(error.to_string()))?;
        *authorization = Some(crate::background_session::WorkUnitContext::new(
            card.id.clone(),
            card.title.clone(),
            card.status.clone(),
        ));
        drop(authorization);
        codelet_tools::broadcast_metadata_update();
        Ok(())
    }
}
