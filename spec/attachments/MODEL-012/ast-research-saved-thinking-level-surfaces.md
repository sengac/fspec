# MODEL-012 AST research: saved thinking level surfaces

## `load_default_thinking_level_opt()` in rust/sessions/src
```
No matches found
```

## `load_default_thinking_level_opt()` in rust/fspec-tui/src
```
rust/fspec-tui/src/app/dispatch_thinking_levels.rs:67:33:load_default_thinking_level_opt()

```

## `save_default_thinking_level($L)` in rust/sessions/src
```
No matches found
```

## `session.set_base_thinking_level($L)` in rust/sessions/src
```
rust/sessions/src/session_creation_helper.rs:244:9:session.set_base_thinking_level(restored_thinking_level)

```

## `session.set_thinking_target($T)` in rust/sessions/src
```
rust/sessions/src/session_creation_helper.rs:232:5:session.set_thinking_target(thinking_target)

```

## `$L.fits_saved_default()` in rust/fspec-tui/src
```
rust/fspec-tui/src/components/thinking_level_dialog/actions.rs:52:28:level.fits_saved_default()

```

## `$L.fits_saved_default()` in rust/sessions/src
```
No matches found
```

## `self.apply_default_thinking_level_if_needed($S)` in rust/fspec-tui/src
```
rust/fspec-tui/src/app/bootstrap.rs:101:21:self.apply_default_thinking_level_if_needed(&session)
rust/fspec-tui/src/app/dispatch_session_chrome.rs:37:13:self.apply_default_thinking_level_if_needed(&session)

```

## `load_default_thinking_level_opt_with_dirs($A, $B)` in rust/sessions/src
```
rust/sessions/src/default_thinking_level_persistence.rs:217:5:load_default_thinking_level_opt_with_dirs(&data_dir, &cwd)

```

## `fn set_model($$$ARGS) -> $R { $$$BODY }` in rust/sessions/src/handle_impl
```
No matches found
```

## Findings
- The saved default is read in three places: the daemon's session creation helper (applies to new, isolated and resumed sessions), the TUI's dialog marker (dispatch_thinking_levels.rs), and the TUI's own TUI-093 apply (dispatch_session_chrome.rs, bootstrap.rs).
- set_model (handle_impl_6.rs) re-caches the ThinkingTarget but never touches the level; MODEL-012 adds the saved-level apply there.
- fits_saved_default gates D in the dialog (thinking_level_dialog/actions.rs) and in set_thinking_level_default (handle_impl_4.rs).
