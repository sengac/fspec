# Research: reverse-session file path depends on how the project path is spelled

Target: `cargo test --profile ci-test -p codelet-fspec --test cli_reverse`
Failing: `scenario_cli_existing_session_detected_prints_suggestions_and_exits_1` (cli_reverse.rs:265)
— expected exit 1 with "Existing reverse session detected", got exit 0 and a fresh gap analysis.
Same failure on clean HEAD. **This is a product bug, not only a test bug.**

## Root cause
`codelet_fspec_core::types::reverse_session::session_path` (`rust/fspec-core/src/types/reverse_session.rs:105`)
hashes the *string form* of `find_project_root(project_root)` and stores the session at
`$TMPDIR/fspec-reverse-<sha256[..12]>.json`. It never canonicalizes the path.

On macOS, temp dirs are reachable as `/var/folders/...` **and** `/private/var/folders/...`
(`/var` is a symlink). The test writes the session using the path returned by `TempDir`
(`/var/...`), while the spawned binary resolves its project root from `current_dir()`, which
the OS reports canonically (`/private/var/...`) → different hash → session "missing".

Verified 2026-10-30 with the debug binary:
```
noncanonical /var/folders/.../tmp.1Enub0fmpb          -> 8ad40b9fe09d
canonical    /private/var/folders/.../tmp.1Enub0fmpb  -> 6414fcb7646b
session written under the noncanonical hash → `fspec reverse` reports "No gaps detected", exit 0
```
Real users hit the same split when they enter a project through a symlink (or different
casing on case-insensitive volumes): an in-progress reverse session becomes invisible.

## Suggested fix
Canonicalize the resolved root (`std::fs::canonicalize`, falling back to the original on
error) before hashing in `session_path`. Add a unit test hashing a symlinked and a real
spelling of the same directory to the same path, and keep the CLI scenario. Consider a
one-time fallback lookup of the legacy (non-canonical) hash so existing sessions are found.
