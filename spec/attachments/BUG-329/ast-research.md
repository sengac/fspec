# AST research: BUG-329

## `fn report_persistence_failure(&self, $$$A) { $$$B }` in rust/agent-loop/src
```
rust/agent-loop/src/background_output.rs:357:5:fn report_persistence_failure(&self, error: &str) {

```

## `$S.is_interrupted.store(true, $O)` in rust/agent-loop/src
```
rust/agent-loop/src/background_output.rs:364:9:self.session

```

## `codelet_core::persistence::load_session($ID)` in rust/agent-loop/src/stream_journal.rs
```
rust/agent-loop/src/stream_journal.rs:70:28:codelet_core::persistence::load_session(self.identity.session_id)

```

## `pub(crate) fn admit(&mut self, $$$A) -> $R { $$$B }` in rust/agent-loop/src/stream_journal.rs
```
rust/agent-loop/src/stream_journal.rs:99:5:pub(crate) fn admit(

```

## `pub fn set_data_directory($$$A) -> $R { $$$B }` in rust/common/src
```
rust/common/src/data_dir.rs:27:1:pub fn set_data_directory(dir: PathBuf) -> Result<(), String> {

```

## `codelet_common::set_data_directory($D)` in rust/agent-loop/tests/response_attempt_rollback.rs
```
rust/agent-loop/tests/response_attempt_rollback.rs:58:5:codelet_common::set_data_directory(directory.path().to_path_buf())
rust/agent-loop/tests/response_attempt_rollback.rs:137:5:codelet_common::set_data_directory(directory.path().to_path_buf())

```

## `create_session_with_provider($$$A)` in rust/agent-loop/tests/response_attempt_rollback.rs
```
rust/agent-loop/tests/response_attempt_rollback.rs:59:20:create_session_with_provider("rollback", directory.path(), "codex")

```

## `BackgroundOutput::with_provider($$$A)` in rust/agent-loop/tests/response_attempt_rollback.rs
```
rust/agent-loop/tests/response_attempt_rollback.rs:62:18:BackgroundOutput::with_provider(background, "codex".into(), None)
rust/agent-loop/tests/response_attempt_rollback.rs:140:18:BackgroundOutput::with_provider(background.clone(), "codex".into(), None)

```
