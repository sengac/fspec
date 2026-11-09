//! BUG-577: the Codex `/models` fetch for the provider list.
//!
//! PROV-176 (product owner, 2026-11-06) fetches `/models` every time the
//! provider list is built. The list must still arrive: it waits at most
//! [`LIST_BUDGET`] for the answer, well under the 10 s RPC deadline, and a
//! list built while a fetch is in flight joins that fetch instead of
//! starting another. A fetch that outlives the budget keeps running on its
//! own thread for the lists that join it; nothing is cached once it ends.
//!
//! Feature: spec/features/codex-models-never-block-the-provider-list-or-replace-the-saved-model.feature

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use super::CodexCatalog;

/// How long building the provider list waits for Codex `/models`.
pub const LIST_BUDGET: Duration = Duration::from_secs(3);

/// Why the provider list has no Codex `/models` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingFailure {
    /// Codex has no ChatGPT tokens (an API key alone cannot fetch).
    NotSignedInWithChatGpt,
    /// The sign-in could not be used or the request failed; the cause.
    Failed(String),
    /// No answer within the list budget.
    NoAnswerInTime(Duration),
}

impl ListingFailure {
    /// The reason the model selector shows in the Codex section header:
    /// about the model list, short, the actionable part first.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::NotSignedInWithChatGpt => {
                "Codex models need ChatGPT sign-in; sign in under /provider".to_string()
            }
            Self::Failed(cause) => format!("model list unavailable: {cause}"),
            Self::NoAnswerInTime(budget) => format!(
                "model list did not arrive in time ({} s); press r to try again",
                budget.as_secs_f32()
            ),
        }
    }

    /// The bare cause, for callers that word it themselves.
    #[must_use]
    pub fn cause(&self) -> String {
        match self {
            Self::NotSignedInWithChatGpt => "Codex is not signed in with ChatGPT".to_string(),
            Self::Failed(cause) => cause.clone(),
            Self::NoAnswerInTime(budget) => format!(
                "the Codex /models request did not answer within {} s",
                budget.as_secs_f32()
            ),
        }
    }
}

type Answer = Result<CodexCatalog, ListingFailure>;

/// One `/models` request and the lists waiting on it.
#[derive(Default)]
struct Flight {
    answer: Mutex<Option<Answer>>,
    answered: Condvar,
}

/// The fetch in flight, if any.
static IN_FLIGHT: Mutex<Option<Arc<Flight>>> = Mutex::new(None);
/// How many provider lists are waiting on a fetch right now.
static WAITING: AtomicUsize = AtomicUsize::new(0);

/// The Codex `/models` answer for one provider list, waiting at most the
/// list budget. Blocking: the provider list is built synchronously.
pub fn fetch_for_listing() -> Answer {
    let flight = join_or_start()?;
    let budget = list_budget();
    let answer = flight.answer.lock().unwrap_or_else(PoisonError::into_inner);
    WAITING.fetch_add(1, Ordering::SeqCst);
    let (answer, _) = flight
        .answered
        .wait_timeout_while(answer, budget, |answer| answer.is_none())
        .unwrap_or_else(PoisonError::into_inner);
    WAITING.fetch_sub(1, Ordering::SeqCst);
    answer
        .clone()
        .unwrap_or(Err(ListingFailure::NoAnswerInTime(budget)))
}

fn join_or_start() -> Result<Arc<Flight>, ListingFailure> {
    let mut slot = IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(flight) = slot.as_ref() {
        return Ok(Arc::clone(flight));
    }
    let flight = Arc::new(Flight::default());
    let fetching = Arc::clone(&flight);
    std::thread::Builder::new()
        .name("codex-models-list".to_string())
        .spawn(move || run(&fetching))
        .map_err(|error| {
            ListingFailure::Failed(format!(
                "the Codex /models request could not start: {error}"
            ))
        })?;
    *slot = Some(Arc::clone(&flight));
    Ok(flight)
}

/// Fetch on this thread's own runtime, end the flight, then publish the
/// answer, so a list built after it ends starts a new fetch (PROV-176).
fn run(flight: &Arc<Flight>) {
    let answer = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            ListingFailure::Failed(format!(
                "the Codex /models request could not start: {error}"
            ))
        })
        .and_then(|runtime| runtime.block_on(super::fetch::fetch_signed_in()));
    {
        let mut slot = IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner);
        if slot
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, flight))
        {
            *slot = None;
        }
    }
    *flight.answer.lock().unwrap_or_else(PoisonError::into_inner) = Some(answer);
    flight.answered.notify_all();
}

#[cfg(not(feature = "test-support"))]
fn list_budget() -> Duration {
    LIST_BUDGET
}

/// Test builds wait as long as the TEST-042 deadlock guard unless a test
/// sets a budget, so a loaded machine never turns a prompt answer into "did
/// not arrive in time".
#[cfg(feature = "test-support")]
fn list_budget() -> Duration {
    test_budget::get().unwrap_or(Duration::from_secs(300))
}

#[cfg(feature = "test-support")]
pub(super) mod test_budget {
    use std::sync::{Mutex, PoisonError};
    use std::time::Duration;

    static BUDGET: Mutex<Option<Duration>> = Mutex::new(None);

    pub(in super::super) fn set(budget: Option<Duration>) {
        *BUDGET.lock().unwrap_or_else(PoisonError::into_inner) = budget;
    }

    pub(super) fn get() -> Option<Duration> {
        *BUDGET.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// How many provider lists are waiting on a Codex fetch.
#[cfg(feature = "test-support")]
pub(super) fn waiting() -> usize {
    WAITING.load(Ordering::SeqCst)
}

/// Whether a Codex fetch for the provider list is in flight.
#[cfg(feature = "test-support")]
pub(super) fn in_flight() -> bool {
    IN_FLIGHT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some()
}
