//! Scheduler module — RPC-058/RPC-059 thin re-export shim.
//!
//! After RPC-058 the engine, state, cron_utils, types, trigger,
//! agent_job, shell_job, catch_up, and job_log modules live in
//! [`codelet_core::scheduler`]. After RPC-059 the `loop_store` module
//! lives in [`codelet_core::loops`]. This shim re-exports them under
//! the historical path so the rest of `codelet-napi` continues to
//! compile unchanged.
//!
//! The thin [`spawn_scheduler`] wrapper preserves the legacy
//! `(project, &runtime_handle)` signature used by
//! `codelet/napi/src/session_hooks.rs` and constructs the
//! [`NapiSchedulerHooks`] adapter that talks to the NAPI
//! [`crate::session_bindings::SessionManager`] singleton via the
//! [`SchedulerHooks`] trait.

pub use codelet_core::scheduler::{
    agent_job, catch_up, cron_utils, crud, engine, evaluate_and_run, evaluate_schedules,
    job_log, shell_job, state, trigger, types, EvaluationResult, Hooks, NoopSchedulerHooks,
    ScheduleEntry, ScheduleTrigger, SchedulerHooks, SchedulerState, SchedulesFile,
};

// RPC-059: loop_store has been lifted into codelet_core::loops. The
// re-export below preserves the public type names; the inner shim
// module preserves callers that use the absolute path
// `crate::scheduler::loop_store::…`.
pub use codelet_core::loops::{IdleCheckFn, LoopEntry, LoopStore};
pub mod loop_store {
    pub use codelet_core::loops::*;
}

use async_trait::async_trait;
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// NAPI-side [`SchedulerHooks`] implementation. Wraps the global
/// `SessionManager` singleton so the lifted engine never has to import
/// `crate::session_bindings` directly.
#[derive(Default)]
pub struct NapiSchedulerHooks;

#[async_trait]
impl SchedulerHooks for NapiSchedulerHooks {
    async fn get_session_count(&self) -> usize {
        #[cfg(not(feature = "noop"))]
        {
            let sm = crate::session_bindings::SessionManager::instance();
            sm.session_count().await
        }
        #[cfg(feature = "noop")]
        {
            0
        }
    }

    async fn get_live_session_ids(&self) -> Vec<Uuid> {
        #[cfg(not(feature = "noop"))]
        {
            let sm = crate::session_bindings::SessionManager::instance();
            sm.live_session_ids().await
        }
        #[cfg(feature = "noop")]
        {
            Vec::new()
        }
    }

    async fn spawn_scheduled_session(&self, trigger: ScheduleTrigger) -> Result<(), String> {
        #[cfg(not(feature = "noop"))]
        {
            let sm = crate::session_bindings::SessionManager::instance();
            sm.spawn_scheduled_session(
                &trigger.session_id.to_string(),
                &trigger.default_model,
                &trigger.project_path,
                &trigger.session_name,
                &trigger.name,
                trigger.role.as_deref(),
                &trigger.prompt,
            )
            .await
            .map_err(|e| e.to_string())
        }
        #[cfg(feature = "noop")]
        {
            let _ = trigger;
            Err("SessionManager not available in noop mode".to_string())
        }
    }

    fn default_model(&self) -> String {
        #[cfg(not(feature = "noop"))]
        {
            crate::session_bindings::SessionManager::instance()
                .get_default_model()
                .unwrap_or_default()
        }
        #[cfg(feature = "noop")]
        {
            String::new()
        }
    }

    async fn find_session_by_schedule_name(&self, schedule_name: &str) -> Option<Uuid> {
        #[cfg(not(feature = "noop"))]
        {
            let sm = crate::session_bindings::SessionManager::instance();
            sm.find_session_by_schedule_name(schedule_name).await
        }
        #[cfg(feature = "noop")]
        {
            let _ = schedule_name;
            None
        }
    }
}

/// Legacy entry point preserved for `codelet/napi/src/session_hooks.rs`.
/// Wraps the new lifted engine with [`NapiSchedulerHooks`].
pub fn spawn_scheduler(project_path: String, handle: &tokio::runtime::Handle) -> JoinHandle<()> {
    let hooks: Arc<dyn SchedulerHooks> = Arc::new(NapiSchedulerHooks);
    engine::spawn_scheduler(project_path, handle, hooks)
}
