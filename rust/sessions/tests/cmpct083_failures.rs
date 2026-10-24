//! Feature: spec/features/cold-resume-compaction-failure-gate.feature

#[path = "cmpct083/application.rs"]
mod application;
#[path = "cmpct083/custom.rs"]
mod custom;
#[path = "cmpct083/fixture.rs"]
mod fixture;
#[path = "cmpct083/negative.rs"]
mod negative;
#[path = "cmpct083/negative_transport.rs"]
mod negative_transport;
#[path = "cmpct083/originals.rs"]
mod originals;
#[path = "cmpct083/runtime.rs"]
mod runtime;
#[path = "cmpct083/transport.rs"]
mod transport;
#[path = "cmpct083/zai.rs"]
mod zai;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_missing_handler_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("missing handler").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_bad_candidate_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("bad candidate").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_timeout_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("timeout").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_stale_generation_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("stale generation").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_durable_write_failure_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("durable write failure").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_structured_replay_gap_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("structured replay gap").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_websocket_upgrade_rejection_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("websocket upgrade rejection").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_incompatible_checkpoint_lineage_blocks_without_fallback() -> fixture::TestResult {
    // @step Given a session with an authoritative committed DAG generation
    // @step When <failure> interrupts the selected compaction or replay path
    // @step Then the operation terminates boundedly with a typed failure
    // @step And the committed generation and immutable originals remain unchanged
    // @step And ordinary inference is blocked with zero alternate requests and no completion notice
    negative::run("incompatible checkpoint lineage").await
}
