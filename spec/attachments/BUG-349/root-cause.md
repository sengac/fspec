# BUG-349 root cause

`Fixture::flush` in `rust/fspec-tui/tests/bug210_progress/support.rs` is the stream barrier. It pushes a `StreamChunk::UserNotification` marker into each session, then waits up to 3 s to see the marker as `Action::ChunkReceived(session, UserNotification { .. })`.
Since TUI-113's transcript routing (`app/dispatch_transcript_live.rs`: `transcript_push_action` / `is_transcript_content`), a `UserNotification` is transcript content. It arrives as `Action::Transcript(TranscriptAction::Live(session, chunk))` instead, never as `ChunkReceived`.
The barrier therefore never clears, and every BUG-210 test fails with `Elapsed(())`. Product code is correct: `CompactionProgressUpdate` is not transcript content and still arrives as `ChunkReceived`.

Fix: the barrier matches the marker on `Action::Transcript(TranscriptAction::Live(..))`.
