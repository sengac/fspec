use std::{future::Future, time::Duration};
use tokio::time::Instant;
use uuid::Uuid;

#[derive(Clone)]
struct Budget {
    operation_id: Uuid,
    started: Instant,
    deadline: Instant,
}

pub fn synthesis_deadline() -> Instant {
    deadline().unwrap_or_else(attempt_deadline)
}

tokio::task_local! {
    static BUDGET: Budget;
    static ATTEMPT_DEADLINE: Instant;
}

pub fn operation_id() -> Option<Uuid> {
    BUDGET.try_with(|budget| budget.operation_id).ok()
}

pub fn deadline() -> Option<Instant> {
    BUDGET.try_with(|budget| budget.deadline).ok()
}

pub fn attempt_deadline() -> Instant {
    ATTEMPT_DEADLINE
        .try_with(|deadline| *deadline)
        .ok()
        .or_else(deadline)
        .unwrap_or_else(|| Instant::now() + crate::interactive::deep_search_wall_clock_timeout())
}

pub fn exploration_deadline() -> Instant {
    let now = Instant::now();
    now + attempt_deadline().saturating_duration_since(now) / 2
}

pub async fn scope<Output>(
    operation_id: Uuid,
    deadline: Instant,
    effective_budget: Duration,
    work: impl Future<Output = Output>,
) -> Output {
    if BUDGET.try_with(|_| ()).is_ok() {
        return work.await;
    }
    BUDGET
        .scope(
            Budget {
                operation_id,
                started: deadline - effective_budget,
                deadline,
            },
            work,
        )
        .await
}

pub async fn attempt<Output>(
    attempt: u32,
    maximum: u32,
    work: impl Future<Output = Output>,
) -> Output {
    let now = Instant::now();
    let deadline = deadline().unwrap_or_else(attempt_deadline);
    let slots = maximum.saturating_sub(attempt).saturating_add(1);
    let until = now + deadline.saturating_duration_since(now) / slots;
    tracing::info!(operation_id = ?operation_id(), attempt, maximum,
        remaining_ms = deadline.saturating_duration_since(now).as_millis(),
        attempt_target_ms = until.saturating_duration_since(now).as_millis(),
        "Compaction candidate attempt started");
    ATTEMPT_DEADLINE.scope(until, work).await
}

pub struct Phase {
    name: &'static str,
    started: Instant,
    budget: Option<Budget>,
}

impl Phase {
    pub fn new(name: &'static str) -> Self {
        let phase = Self {
            name,
            started: Instant::now(),
            budget: BUDGET.try_with(Clone::clone).ok(),
        };
        if let Some(budget) = &phase.budget {
            tracing::info!(
                operation_id = %budget.operation_id,
                phase = name,
                effective_budget_ms = budget.deadline.saturating_duration_since(budget.started).as_millis(),
                remaining_ms = budget.deadline.saturating_duration_since(phase.started).as_millis(),
                "Compaction phase started"
            );
        }
        phase
    }
}

impl Drop for Phase {
    fn drop(&mut self) {
        if let Some(budget) = &self.budget {
            tracing::info!(
                operation_id = %budget.operation_id,
                phase = self.name,
                phase_elapsed_ms = self.started.elapsed().as_millis(),
                operation_elapsed_ms = budget.started.elapsed().as_millis(),
                remaining_ms = budget.deadline.saturating_duration_since(Instant::now()).as_millis(),
                "Compaction phase finished or cancelled"
            );
        }
    }
}
