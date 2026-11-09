# AST research: BUG-326

## `current.token_usage.current_context_tokens = $E` in rust/core/src/persistence
```
rust/core/src/persistence/compaction_commit.rs:180:9:current.token_usage.current_context_tokens = admitted.input_tokens
rust/core/src/persistence/request_settlement.rs:19:9:current.token_usage.current_context_tokens = usage.map_or(0, |usage| usage.total_tokens)

```

## `self.token_usage.current_context_tokens = $E` in rust/core/src/persistence
```
rust/core/src/persistence/manifest_types.rs:138:9:self.token_usage.current_context_tokens = input

```

## `tracker.input_tokens = saved.current_context_tokens` in rust/sessions/src
```
rust/sessions/src/session_restore.rs:179:17:tracker.input_tokens = saved.current_context_tokens

```

## `self.token_tracker.input_tokens = saved.current_context_tokens` in rust/cli/src/session
```
rust/cli/src/session/restore_compaction.rs:89:9:self.token_tracker.input_tokens = saved.current_context_tokens

```

## `input_tokens: convert(saved.current_context_tokens)?` in rust/sessions/src
```
No matches found
```

## `snapshot.context_fill.effective_tokens = $E` in rust/sessions/src
```
rust/sessions/src/committed_publication.rs:109:5:snapshot.context_fill.effective_tokens = f64::from(tokens.input_tokens)

```

## `tracker.output_tokens = saved.cumulative_billed_output` in rust/sessions/src
```
rust/sessions/src/session_restore.rs:180:17:tracker.output_tokens = saved.cumulative_billed_output

```

## `self.output_tokens = cumulative_output` in rust/core/src/compaction
```
rust/core/src/compaction/input_accounting.rs:60:9:self.output_tokens = cumulative_output
rust/core/src/compaction/model.rs:173:9:self.output_tokens = cumulative_output

```

