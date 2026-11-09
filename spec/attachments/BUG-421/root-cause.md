# BUG-421 root cause

Integration tests build the project path for `list_sessions` as

    &std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()

If `current_dir()` fails (deleted cwd, permission error) the Err is swallowed and the empty string is
passed. `SessionManager::list_sessions("")` means "all projects", so the test silently runs a different
query than the one it names. This is a silent default on input, which the no-fallback rule forbids.

Fix: use `.expect("current dir")` or, better, the project path the test created the session with.
