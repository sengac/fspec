# REFAC-057 AST research

- Fixtures (HEAD lines 48-140) and `read_source` (HEAD 362-374) are used only by this file's tests.
- `feed_loop_stream` calls `StreamOutput::emit` (trait method, rust/cli/src/interactive/output.rs:283), so the
  support module imports `codelet_cli::interactive::StreamOutput`; the tests do not call trait methods directly.
- Noticed, filed separately: `fresh_background_session` discards `set_data_directory`'s Result (BUG-406 scope) and
  `read_source` falls back to the relative path when the manifest dir has no grandparent.
