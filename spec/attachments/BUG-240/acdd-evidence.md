# BUG-240 ACDD evidence

## Scope and implementation

The root harness field attributes the calling session, never the spawned or
targeted session. Shared model-visible schema and AgentManager descriptions now
state that explicitly. The management description gives the concrete example of
orchestrating AUTH-001 while assigning AUTH-002 through role/message text, and
instructs workers to attribute their own calls. Documentation uses the same
example. No admission behavior, authorization rule, or child initialization API
was changed.

## Specification and RED

Example Mapping, a user story and architecture notes were recorded through fspec
before generating `agent-manager-caller-attribution.feature`. Three concrete
scenarios were added and validated before entering testing. No temporal or step
validation bypass flags were used.

Before production edits, the compiled guidance regression failed on the old
description:

```text
Required harness attribution: an existing work-unit ID in the invoking project,
or exact NONE to explicitly clear activity (not retain it). Omission, null and
empty strings are invalid. This is not a business target or permission change.
```

The failing assertion required explicit calling/spawned/targeted-session wording.
Both unchanged runtime-isolation scenarios passed in that same run. This is the
expected distinction: the defect was ambiguous guidance, not broken admission.
Evidence: `/tmp/bug240-red-confirmed.log` (2 runtime tests pass; 1 guidance test
fails). The initial split test layout was consolidated into one test file to
satisfy fspec's one-feature/one-test-file gate without weakening assertions.

## GREEN and regression checks

All commands use the bounded `ci-test` profile and `-j 2`:

- `cargo test --profile ci-test -p codelet-sessions --test bug240_management_attribution -j 2`:
  **3/3 PASS**, `/tmp/bug240-post-consolidation.log`.
- `cargo test --profile ci-test -p codelet-tools --test tool025_schema_contract -p codelet-sessions --test tool026_admission --test sess003_work_unit_focus -p codelet-agent-loop --test agent_manager_busy_messages -j 2 --no-fail-fast`:
  **22/22 PASS**, `/tmp/bug240-regressions.log` (4 schema, 7 admission, 3 focus,
  8 management-message tests).
- `cargo test --profile ci-test -p codelet-tools --lib agent_manager -j 2`:
  **78/78 PASS**, `/tmp/bug240-manager-unit-tests.log`.
- Final combined rerun of the three new scenarios and all 22 related regression
  tests after the BUG-239 shared privacy changes: **25/25 PASS**,
  `/tmp/bug240-final-regressions.log`.
- `cargo check --profile ci-test -p codelet-tools -p codelet-sessions -p codelet-agent-loop -j 2`:
  **PASS**, `/tmp/bug240-check.log`.
- `cargo clippy --profile ci-test -p codelet-tools -p codelet-sessions -p codelet-agent-loop -j 2`:
  **PASS**, `/tmp/bug240-clippy.log`.
- `cargo clippy --profile ci-test -p codelet-sessions --test bug240_management_attribution -j 2`:
  **PASS**, `/tmp/bug240-test-clippy.log`. Importing the existing shared fixture
  emits nine unused-helper warnings; no lint suppression was added.
- Scoped rustfmt check and `git diff --check`: **PASS**.

The new runtime tests use real session managers and temporary project files via
the existing fixture and test helpers. They exercise actual AgentManager spawn,
worker-owned admission, status, role, message and close operations. A strict
dynamic boundary verifies raw business JSON before typed deserialization, so
unknown-field dropping cannot mask a failure to strip root attribution. Audit
arguments retain the root field. Per-session handler guards unregister test
handlers, and message receipt is asserted rather than silently skipped.

## Specifications and coverage

- `fspec validate`: **all 2065 feature files pass**.
- Scoped feature tag validation: **PASS**.
- `fspec show-coverage agent-manager-caller-attribution`: **3/3 fully covered**,
  exactly one scenario-specific test mapping per scenario in one test file.
- `fspec audit-coverage agent-manager-caller-attribution`: **9/9 paths valid**.
- Repository-wide `fspec validate-tags` still reports **526 unrelated feature
  files with existing tag violations**. No unrelated tags were modified.

Coverage generation was run against an isolated copy of this feature and the
fspec-generated sidecar was copied back, avoiding creation of unrelated missing
coverage files. All mappings were created through fspec.

## Boundaries

The shared runtime path is integration-tested. Source review confirms that
provider agents use the shared AgentManagerTool description and that legacy NAPI
registers its own business handler rather than a duplicate schema. NAPI runtime
parity was not tested or claimed. These tests are not a replay of the original
live incident. No workspace-wide Cargo test, commit, branch, reset, or unrelated
staged-change rewrite was performed.
