use super::{registry, validate_card, AttributionError, InvocationReceipt, WorkUnitAttribution};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub struct AuxiliaryScope(Uuid);

impl Drop for AuxiliaryScope {
    fn drop(&mut self) {
        registry::unregister_session(self.0);
    }
}

pub fn register_auxiliary(id: Uuid, project: PathBuf) -> Result<AuxiliaryScope, AttributionError> {
    let parent = crate::execution_context::current_invocation();
    let owner = parent.as_ref().map(|receipt| receipt.session_id);
    let initial = parent.map_or(WorkUnitAttribution::None, |receipt| receipt.attribution);
    register(id, project, owner, initial)
}

pub fn register_auxiliary_for_owner(
    id: Uuid,
    project: PathBuf,
    owner: Uuid,
) -> Result<AuxiliaryScope, AttributionError> {
    register(id, project, Some(owner), WorkUnitAttribution::None)
}

fn register(
    id: Uuid,
    project: PathBuf,
    owner: Option<Uuid>,
    initial: WorkUnitAttribution,
) -> Result<AuxiliaryScope, AttributionError> {
    let project = project
        .canonicalize()
        .map_err(|error| AttributionError(format!("cannot resolve auxiliary project: {error}")))?;
    let authorization = match owner {
        Some(owner) => {
            let (trusted, authorization) = registry::trusted_context(owner)?;
            if trusted.canonicalize().ok().as_ref() != Some(&project) {
                return Err(AttributionError(
                    "auxiliary project differs from trusted owner".into(),
                ));
            }
            authorization
        }
        None => id,
    };
    let validator: registry::ContextValidator = Arc::new(move || {
        if let Some(owner) = owner {
            registry::trusted_context(owner).map_err(|error| error.to_string())?;
        }
        Ok(())
    });
    let state = Arc::new(Mutex::new((0_u64, 0_u64, initial)));
    registry::register(
        id,
        project.clone(),
        authorization,
        Arc::new(move |attribution| {
            let mut state = state
                .lock()
                .map_err(|_| "auxiliary focus lock poisoned".to_owned())?;
            validate_card(&project, &attribution).map_err(|error| error.to_string())?;
            let admission = state
                .0
                .checked_add(1)
                .ok_or("auxiliary admission exhausted")?;
            let revision = if state.2 == attribution {
                state.1
            } else {
                state
                    .1
                    .checked_add(1)
                    .ok_or("auxiliary revision exhausted")?
            };
            *state = (admission, revision, attribution.clone());
            Ok(InvocationReceipt {
                session_id: id,
                project: project.clone(),
                attribution,
                admission,
                focus_revision: revision,
            })
        }),
        validator,
    );
    Ok(AuxiliaryScope(id))
}
