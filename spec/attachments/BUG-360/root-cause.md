# Root cause: the status gate silently skips unreadable or unparsable feature files

`rust/fspec-core/src/commands/update_work_unit_status/step_docstrings.rs:199` `matching_features`:
- `let Ok(entries) = std::fs::read_dir(&features_dir) else { return out; }`: an unreadable `spec/features` returns no features, so every gate that depends on them passes vacuously.
- `let Ok(content) = std::fs::read_to_string(&path) else { continue; }`: an unreadable feature file is skipped.
- `let Ok(feature) = parse_feature_lenient(&content) else { continue; }`: an unparsable feature file is skipped.
- `.flatten()` on the directory entries drops entry errors.

## Effect
A work unit whose feature file can't be read or parsed is moved through status gates (temporal ordering, one test file, step docstrings) as if it had no feature. The gate passes when it should refuse.

## Fix
- Return `Result<Vec<MatchedFeature>, _>`, and propagate read_dir, entry, read and parse errors as a typed error naming the path, so the status move fails with an actionable message.
- A missing `spec/features` directory is an explicit error too, unless the caller's contract says a project without features is valid. Decide that during specifying, and never return an empty list silently.
- Test each case: unreadable directory, unreadable file, unparsable file.

Found by the agent that worked BUG-355 through BUG-348 (2026-11-02).
