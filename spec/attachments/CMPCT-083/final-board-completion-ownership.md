# Final board completion ownership

Scope: the 27 CMPCT cards active at the start of this pass. Existing staged,
unstaged and untracked work is retained; earlier completion wording is not
substituted for current acceptance evidence. Historical sessions cannot be
resumed, so replacement owners have explicit nonoverlapping write scopes.

| Owner | Exclusive write scope |
| --- | --- |
| `7be436a7` | `rust/core/**`, core-specific features and sidecars |
| `bee8c5ed` | `rust/providers/**`, provider-specific features and sidecars |
| `77e0a104` | `rust/cli/**`, CLI-specific features and sidecars |
| `380c3330` | `rust/agent-loop/**`, `rust/tools/**`, loop-specific specifications |
| `ced1485e` | `rust/sessions/**` except gate tests, `rust/fspec-tui/**`, `rust/rpc-types/**`, corresponding specifications |
| `a3428a51` | `rust/patches/rig-core/**`, Rig-specific specifications |
| `414c4991` | `rust/sessions/tests/cmpct083*`, gate specifications and audit |
| `d14d496e` | Independent read-only acceptance and traceability audit |
| Coordinator | Board transitions, cross-owner coordination, final evidence |

## Completion sequence

1. Reconcile retained source, specifications, historical RED evidence and current
   validation results; distinguish genuine unfinished behavior from stale notes.
2. Close real gaps with specification first, executed failing test second, and
   production repair last. Preserve all unrelated work and safety assertions.
3. Verify bounded scoped Cargo targets, formatting, compilation, Clippy, feature
   syntax, scoped registered tags and normal exact coverage-link validation.
4. Independently review acceptance semantics and rerun the strengthened genuine
   cold-process/no-fallback gate against the integrated final source.
5. Attach evidence and advance every active card through remaining workflow
   states in dependency order, without temporal or step-validation bypasses.

Source owners must coordinate cross-crate fixes through the coordinator. Disk
headroom starts at 23 GiB, so builds use existing bounded `ci-test` targets.
No staging, commits, branch creation or unrelated artifact deletion is requested.

Initially active: CMPCT-052, 054, 055, 056, 057, 058, 059, 060, 061, 062,
064, 065, 066, 067, 068, 069, 070, 071, 072, 074, 075, 078, 079, 080,
081, 082 and 083. This ownership record is not a completion claim.
