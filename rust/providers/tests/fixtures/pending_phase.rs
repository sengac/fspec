//! Wait for a Codex request to reach the network phase a scenario selects,
//! without a fixed sleep: the first request also builds the token counter
//! that admission uses, which takes a few hundred milliseconds in an
//! unoptimised build.

use futures::{Stream, StreamExt};

/// Poll `stream` until `ready` holds. Fails if the stream yields anything
/// first (the request must still be pending) or `ready` does not hold
/// within 5 s.
pub async fn pending_until<S: Stream + Unpin>(
    stream: &mut S,
    mut ready: impl FnMut() -> Result<bool, String>,
) -> Result<(), String> {
    // TEST-042: the stream is polled between checks of `ready`, with no
    // waiting window, under the deadlock guard.
    let mut failure = None;
    let mut next = stream.next();
    let reached = codelet_test_helpers::pending_until(
        "the request reaches the selected network phase",
        &mut next,
        || match ready() {
            Ok(ready) => ready,
            Err(error) => {
                failure = Some(error);
                true
            }
        },
    )
    .await;
    drop(next);
    if let Some(error) = failure {
        return Err(error);
    }
    reached.map_err(|failure| {
        format!("the stream yielded while the request should be pending: {failure}")
    })
}
