# TOOL-026 ACDD evidence

## Specification and workflow

The existing Example Mapping and seven acceptance scenarios were reviewed before
implementation. The three outstanding questions were resolved using the explicit
implementation decisions already recorded in
`docs/research/session-work-unit-focus.md`, not attributed to a new human answer.
AST research is attached in `ast-research-admission.md`. Estimate: eight points.

The current wire contract requires top-level `work_unit_id`, without wrapper
compatibility. The logs below record prior implementation validation, not a rerun
of this revised contract; this documentation update establishes no new test results.

The card followed specifying -> testing -> implementing, returned to testing for
security refinements, then implementing -> validating -> done. Production changes
were made only after behavioral RED. All seven tests remain in one integration
target, with exact step comments immediately before the corresponding actions or
assertions. Support modules contain additional checks, not unmapped test cases.

## Behavioral RED

`/tmp/tool026-red.log` records seven compiled, failing acceptance tests before
runtime implementation. They expose missing invoking receipts, unstripped metadata,
unvalidated side effects, missing focus updates, and completion admission bypass.
Earlier fixture compile errors are not counted as behavioral evidence.

The expanded strict typed, MCP, facade and Rhai checks then passed, while
`/tmp/tool026-refinement-red.log` exposed foreign-project auxiliary registration.
`/tmp/tool026-policy-red.log` also exposed same-scope schema-only middleware silently
preventing runtime policy installation. Both regressions preceded their fixes.

## Implementation and wiring

- Shared admission parses required top-level `work_unit_id`, rejects malformed,
  missing, unknown and foreign-project attribution, then strips only reserved
  root `work_unit_id` before ToolDyn/typed/MCP/facade/Rhai delegation. Existing
  business root `work_unit_id` schemas are rejected as reserved-name collisions;
  nested business targets and `target_work_unit_id` remain unchanged.
- Trusted managed contexts capture project and weak owning-manager identity.
  Expired or unknown invoking contexts fail closed, without guessing the global
  active session or using business target arguments as ownership.
- Shared focus admission runs before execution. Task-local receipts retain
  admission order and explicit attribution for BUG-231 reconciliation. Execution
  failure does not undo accepted activity; late results cannot resurrect it.
- Original arguments remain available in task-local audit context and unchanged
  provider history, including original tool-call/result correlation identities.
- Auxiliary contexts maintain independent activity, validate the inherited project,
  and retain authorization identity. Automatic compaction explicitly supplies its
  owner even without a current tool invocation. Closing the owner rejects further
  auxiliary calls instead of falling back to unrestricted execution.
- Existing stage and isolated-worktree checks retain their permission binding
  through NONE and attribution changes. Session destruction cleans registration.
- Runtime configuration is shared by ordinary CLI execution, dynamic MCP additions,
  completion tools, both DeepSearch front doors, and compaction. Incompatible
  same-scope middleware configuration is rejected rather than silently accepted.
  TOOL-025's raw/hosted-native rejection remains enforced.

Scripted prerequisite fixtures were updated to supply explicit metadata and trusted
standalone contexts where they intentionally instantiate sessions without a manager.
No production bypass or schema-only test fallback was introduced.

## Final GREEN and quality

All Cargo invocations use explicit packages, named integration targets or `--lib`,
`--profile ci-test`, and `-j 8`; no unscoped/workspace test run was used.

- `codelet-sessions --test tool026_admission`: seven pass; final log
  `/tmp/tool026-final-green.log`. Includes property-based preservation checks,
  strict typed and actual loopback MCP execution, real Rhai/facade file reads,
  independent auxiliary/automatic ownership, permission checks, delayed failure,
  duplicate reconciliation, hot tools and completion rejection.
- OpenAI and Anthropic each execute invalid-then-valid tool calls over loopback
  HTTP in both streaming and nonstreaming modes, switching providers on the same
  managed session. Subsequent wire requests retain original attributed history
  and matching result identities. These checks run inside the seventh scenario.
- BUG-231 and SESS-003 regression targets: nine and three pass;
  `/tmp/tool026-focus-regressions-final.log`.
- TOOL-025 schema/provider targets: four and two pass;
  `/tmp/tool026-tool-regressions.log`, `/tmp/tool026-provider-regressions.log`.
- Real Codex accepted-completion batch: one pass;
  `/tmp/tool026-completion-regressions.log`.
- Codex history/reload: two and three pass;
  `/tmp/tool026-history-regressions.log`, `/tmp/tool026-reload-regressions.log`.
- Scoped check succeeds for tools, sessions, agent-loop, CLI and providers:
  `/tmp/tool026-check.log`. NAPI check succeeds with seven existing warnings:
  `/tmp/tool026-napi-check-final.log`.
- Scoped production Clippy succeeds: `/tmp/tool026-clippy-final.log`.
  Acceptance-target Clippy succeeds: `/tmp/tool026-test-clippy-verified.log`.
  Reused BUG-231 support modules produce nine unused-helper warnings; no lint is
  disabled. Redundant conversions and lock-across-await warnings were corrected.
- Targeted rustfmt check and `git diff --check` pass. New Rust modules and the
  generated coverage sidecar remain below 300 lines.
- All seven scenarios have exactly one test mapping plus implementation links:
  100% scenario coverage, not a claim of measured line coverage. Coverage audit
  reports all 20 referenced files/mappings valid.
- All 2,053 feature files validate. This feature's tags validate. Global tag
  validation retains 526 existing violations elsewhere (1,527 passing features).

## Broader validation limitations

The broader five-package library run is recorded in
`/tmp/tool026-library-regressions.log`. CLI's 147 and sessions' 56 tests pass.
Agent-loop has one unrelated SessionSearch description assertion failure; providers
have four Codex default-capacity failures (zero versus 272000). Three existing
subordinate relay tests stall and were terminated. The bounded tools library run
excluding that ten-test relay module passes all remaining 960 tests:
`/tmp/tool026-tools-library.log`.

The broader compaction targets in `/tmp/tool026-compaction-regressions.log` retain
the fixture calibration failure already documented by TOOL-025: 11371 versus
90336 in `support/bug210_completion_metrics.rs`, before admission/execution setup.
Four budget-recovery scenarios and the real Anthropic synthesis scenario pass;
six cases fail this calibration. The unrelated Codex physical-capacity boundary
case stalls and was terminated. No token-estimation, description, capacity-default
or relay changes were made to conceal these baseline issues.

Independent read-only audits found no remaining model-facing execution path outside
configured Rig middleware. Legacy Runner references are tests only; provider-native
completion adapters return call data, Rhai execution delegates through ToolDyn, and
tool-free synthesis rejects unexpected calls. No Git commit or branch was created.
