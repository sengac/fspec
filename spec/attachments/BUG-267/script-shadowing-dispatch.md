# BUG-267: Built-in-name script shadowing and real dispatch

## Finding

Source-confirmed selection/dispatch mismatch. Do not claim the latest routing
extraction introduced it without a behavioral baseline.

## Evidence

- `rust/providers/src/manager.rs:55` intentionally lets registered scripts
  shadow built-in names, returning Custom(claude) for a script named claude.
- `rust/providers/src/custom/management.rs:451` keeps no facade override when
  a scripted config has none explicitly set.
- `rust/agent-loop/src/agent_loop/routing.rs:7` reduces the implementation to
  a provider/facade string; `providers.rs:18` matches claude as the native branch.
- `rust/providers/src/manager.rs:817` then rejects a non-Claude enum.
- `rust/providers/tests/custom_provider_script_shadowing_tests.rs:112` tests
  resolution only; PROV-161 scripted dispatch coverage uses noncolliding slugs
  or direct construction rather than this shared dispatch composition.

## Acceptance/test direction

Create real project/global script fixtures using colliding built-in slugs and
run through the shared background loop. Assert the selected script receives the
request, native getters are not used accidentally, and armed native Done can
complete the task. Cover explicit facade selection, aliases and
FSPEC_DISABLE_SCRIPT_SHADOWING without weakening credential or admission checks.
Preserve typed implementation identity across routing instead of guessing from
a name string. Include normal noncolliding custom and true native controls.

Related contracts: PROV-085, PROV-161, CONT-009.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
