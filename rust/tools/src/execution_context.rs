use std::{future::Future, path::PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkUnitAttribution {
    Absent,
    Card(String),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationReceipt {
    pub session_id: Uuid,
    pub project: PathBuf,
    pub attribution: WorkUnitAttribution,
    pub admission: u64,
    pub focus_revision: u64,
}

tokio::task_local! {
    static INVOCATION: InvocationReceipt;
}

pub async fn scope_invocation<T>(
    receipt: InvocationReceipt,
    operation: impl Future<Output = T>,
) -> T {
    INVOCATION.scope(receipt, operation).await
}

pub fn current_invocation() -> Option<InvocationReceipt> {
    INVOCATION.try_with(Clone::clone).ok()
}
