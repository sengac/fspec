use super::{AttributionError, InvocationReceipt, WorkUnitAttribution};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub type AdmissionHandler =
    Arc<dyn Fn(WorkUnitAttribution) -> Result<InvocationReceipt, String> + Send + Sync>;
pub type ContextValidator = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

#[derive(Clone)]
struct Context {
    handler: AdmissionHandler,
    authorization: Uuid,
    project: PathBuf,
    validator: ContextValidator,
}

pub(super) fn trusted_context(id: Uuid) -> Result<(PathBuf, Uuid), AttributionError> {
    let context = context(id)?;
    Ok((context.project, context.authorization))
}

fn context(id: Uuid) -> Result<Context, AttributionError> {
    let context = CONTEXTS
        .get(&id)
        .map(|context| context.clone())
        .ok_or_else(|| AttributionError(format!("unknown invoking context {id}")))?;
    (context.validator)().map_err(AttributionError)?;
    Ok(context)
}

static CONTEXTS: Lazy<DashMap<Uuid, Context>> = Lazy::new(DashMap::new);

pub fn register_session(
    id: Uuid,
    project: PathBuf,
    handler: AdmissionHandler,
    validator: ContextValidator,
) {
    register(id, project, id, handler, validator);
}

pub(super) fn register(
    id: Uuid,
    project: PathBuf,
    authorization: Uuid,
    handler: AdmissionHandler,
    validator: ContextValidator,
) {
    CONTEXTS.insert(
        id,
        Context {
            handler,
            authorization,
            project,
            validator,
        },
    );
}

pub fn unregister_session(id: Uuid) {
    CONTEXTS.remove(&id);
}

pub fn authorization_identity(id: Uuid) -> Uuid {
    CONTEXTS
        .get(&id)
        .map_or(id, |context| context.authorization)
}

pub(super) fn admit(
    id: Uuid,
    attribution: WorkUnitAttribution,
) -> Result<InvocationReceipt, AttributionError> {
    let context = context(id)?;
    let receipt = (context.handler)(attribution.clone()).map_err(AttributionError)?;
    if receipt.session_id != id || receipt.attribution != attribution {
        return Err(AttributionError(
            "invoking context returned a mismatched receipt".into(),
        ));
    }
    Ok(receipt)
}
