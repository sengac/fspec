//! Feature: spec/features/durable-input-admission.feature

#[path = "bug284/boundaries.rs"]
mod boundaries;
#[path = "bug284/cancellation.rs"]
mod cancellation;
#[path = "bug284/continuation.rs"]
mod continuation;
#[path = "bug284/fixture.rs"]
mod fixture;
#[path = "bug284/historical.rs"]
mod historical;
#[path = "bug284/pending_crash.rs"]
mod pending_crash;
#[path = "../../sessions/src/replay_content.rs"]
pub mod replay_content;
#[path = "../../sessions/src/replay_media.rs"]
pub mod replay_media;
#[path = "../../sessions/src/replay_transcript.rs"]
pub mod replay_transcript;
#[path = "bug284/retention.rs"]
mod retention;
#[path = "bug284/setup_failure.rs"]
mod setup_failure;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;
