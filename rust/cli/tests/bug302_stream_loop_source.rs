//! Feature: spec/features/stream-loop-source-guards.feature
#![allow(clippy::expect_used)]

#[path = "support/stream_loop_source.rs"]
mod stream_loop_source;

#[test]
fn source_guards_see_code_moved_into_stream_loop_modules() {
    // @step Given the stream loop is split across stream_loop.rs and stream_loop/*.rs
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interactive");
    assert!(dir.join("stream_loop.rs").is_file());
    assert!(dir.join("stream_loop/final_completion.rs").is_file());

    // @step When a source guard reads the stream-loop source
    let source = stream_loop_source::stream_loop_source();

    // @step Then it sees the FinalResponse settle code from stream_loop/final_completion.rs
    assert!(source.contains("decide_continuation("));
    assert!(source.contains("emit_done_with_stop_reason"));

    // @step And stream_loop.rs comes first followed by the module files in name order
    let facade = source
        .find("pub async fn run_agent_stream")
        .expect("stream_loop.rs facade");
    let assistant = source
        .find("pub(super) fn handle<Response, Output: StreamOutput>(")
        .expect("stream_loop/assistant.rs");
    let completion = source
        .find("decide_continuation(")
        .expect("stream_loop/final_completion.rs");
    assert!(facade < assistant && assistant < completion);
}
