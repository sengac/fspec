# BUG-284 ACDD layout review

Read-only review of `/home/rquast/projects/fspec`; no repository, feature, test, card or native-session edits; no builds or tests run. Native owner remains `session22032f8b-b6b7-45ea-a47f-5b25017b31f4`. Keep BUG-284 in Testing. The approved mechanical CLI extraction is separate from this mapping repair.

## Smallest coherent split

**Five feature files total (four new) is the minimum without moving or misrepresenting existing test bodies:** the nine mappings already span five physical files, and the gate allows one physical mapped file per feature. Keep the current integration-test target and module structure. Helpers may stay shared; mappings must point to the child files containing the actual tests.

Keep every existing scenario title, keyword, step text, and test body unchanged. Copy the original feature tags, Background, seven rules, eight examples, and architecture obligations into each new feature, or retain the common context verbatim with an explicit pointer to the original feature. Each feature must retain feature-level `@BUG-284` as its first/only work-unit ID tag. The original architecture sentence saying CLI decomposition approval is pending is stale: approval was given on 2026-10-28 at 22:01:24 UTC. Correct that sentence only, preserving the other obligations.

Paths below are under `spec/features/`; modules are under `rust/agent-loop/tests/bug284/`. Ranges are the current sidecar ranges, before any future tests are inserted.

| Feature file | Physical test file | Exact scenarios retained / moved |
|---|---|---|
| `durable-input-admission.feature` (retain existing path) | `boundaries.rs` | **Cancellation during original persistence preserves audit without conversation membership** (8-63); **Cancellation while awaiting the runtime lock does not admit the input** (65-98); **Interrupted execution cannot revoke an already admitted input** (unmapped; add real test here) |
| `submission-cancellation.feature` (new) | `cancellation.rs` | **Six supervisor inputs canceled before admission remain audit only** (7-40); **Two supervisor inputs submitted during interruption remain audit only** (42-75); **Cancellation during a prompt hook preserves audit without conversation membership** (77-104) |
| `admitted-input-setup-failure.feature` (new) | `setup_failure.rs` | **Provider setup and pre-prompt compaction failure cannot split admitted history** (8-63) |
| `submission-receipt-recovery.feature` (new) | `pending_crash.rs` | **Process death after queue acknowledgment retains the pending submission** (19-90); **Submission identity survives retries images and identical prompt text** (unmapped); **Durability acknowledgment loss never grants unresolved execution authority** (unmapped); **Process death preserves input disposition without replaying effects** (unmapped). Add those three real tests here, with helpers elsewhere as needed. |
| `admitted-history-continuation.feature` (new) | `historical.rs` | **Untouched accepted history remains eligible for selected compaction** (12-34); **Legacy divergent histories and altered ancestry remain fail closed** (36-114); **Cancellation and cold continuation survive successive automatic compactions** (unmapped; add real test here) |

Counts immediately after the layout-only split: **14 scenarios, 9 mapped / 5 unmapped, 5 distinct physical mapped files**, with per-feature counts 3/2, 3/3, 1/1, 4/1, 3/2 (total/mapped). Do not create dummy mappings for the five missing tests. Keep files below the AGENTS.md 300-line limit by extracting helpers, not by moving mapped test bodies to another file behind a wrapper. If meaningful tests cannot fit coherently in these five modules, an additional feature/module pair is preferable to violating that rule; five is the minimum layout, not a requirement to cram tests.

## Exact five outstanding acceptance obligations

These must survive the split verbatim and receive executable assertions, not comments alone:

```gherkin
Scenario: Interrupted execution cannot revoke an already admitted input
  Given a submitted input has won durable admission and runtime installation
  When execution is interrupted before provider dispatch
  Then the admitted input remains in runtime and cold replay exactly once
  And a fresh subsequent input may continue without replaying the old input as new work
  And truthful durable cancellation evidence is model visible before subsequent inference

Scenario: Submission identity survives retries images and identical prompt text
  Given two intentional submissions have identical text and image payloads
  When one receipt is retried and a conflicting identity is attempted
  Then the two distinct originals remain recoverable and the retry adds no duplicate
  And the conflicting identity or unknown receipt version blocks inference
  And garbage collection preserves pending receipt and recovery spool blob evidence
  And fork or merge cannot grant execution rights from pending or canceled receipts

Scenario: Durability acknowledgment loss never grants unresolved execution authority
  Given an input is being promoted at a manifest durability boundary
  When durability acknowledgment is lost after rename or storage remains unavailable
  Then the same receipt identity is settled or inference remains blocked
  And the original evidence is preserved without requeueing it under a new identity
  And a local attempt retry cannot be mistaken for a new externally identified submission

Scenario: Process death preserves input disposition without replaying effects
  Given an input is pending admitted installed or beginning provider dispatch
  When the process dies and persisted state is reopened
  Then only admitted inputs restore into the conversation exactly once
  And pending and canceled inputs remain recoverable without automatic execution
  And uncertain dispatch blocks continuation until explicit recovery records a truthful model visible disposition

Scenario: Cancellation and cold continuation survive successive automatic compactions
  Given a real queued conversation has completed parallel tool effects once
  When canceled inputs and fresh admitted inputs span two automatic compactions and cold continuations
  Then each committed generation retains exact original outcomes and valid DAG ancestry
  And no canceled instruction or completed tool effect executes again
  And admitted cancellation and failure dispositions remain model visible after both compactions
```

## Native Fspec command arguments actually supported

These are dispatcher `command` values and JSON payload shapes from `rust/fspec-core/src/commands/*.rs`. Use explicit complete `spec/features/<name>.feature` paths for `feature`/`file`; use a bare basename for `featureName`. Do not assume the different parameter names are interchangeable.

| Command | Payload |
|---|---|
| `create-feature` | `{"name":"Submission cancellation"}` → creates `submission-cancellation.feature` plus a template sidecar; refuses overwrite |
| `add-background` | `{"feature":"spec/features/submission-cancellation.feature","text":"As a agent operator\nI want to cancel submitted work without losing its recovery evidence or corrupting the admitted conversation\nSo that compaction and cold continuation preserve strict history and never repeat completed effects"}` |
| `add-architecture` | `{"feature":"spec/features/<name>.feature","text":"<complete retained architecture text>"}` (replaces the feature doc string) |
| `add-tag-to-feature` | `{"file":"spec/features/<name>.feature","tags":["@phase-1","@context-management","@agent-core","@persistence","@session-management","@high","@integration-test","@BUG-284"]}` |
| `remove-tag-from-feature` | `{"file":"spec/features/<name>.feature","tags":["@component","@feature-group","@critical"]}`; **whole-line matching only**, so this does not remove tags from the combined template tag line unless formatting first separates them; inspect the result |
| `add-scenario` | `{"feature":"spec/features/<name>.feature","scenario":"<exact title>"}` (adds three placeholder steps; does not update sidecar) |
| `update-scenario` | `{"feature":"spec/features/<name>.feature","oldName":"[Scenario name]","newName":"<exact title>"}` (renames matching sidecar entry too, best effort) |
| `update-step` | `{"feature":"spec/features/<name>.feature","scenario":"<exact title>","currentStep":"[precondition]","text":"<exact Given text>","keyword":"Given"}`; similarly replace `[action]` / `[expected outcome]` |
| `add-step` | `{"feature":"spec/features/<name>.feature","scenario":"<exact title>","type":"And","text":"<exact text without keyword>"}` |
| `delete-scenario` | `{"feature":"spec/features/<name>.feature","scenario":"<exact title>"}` (also removes sidecar scenario, best effort) |
| `link-coverage` | `{"featureName":"submission-cancellation","scenario":"Six supervisor inputs canceled before admission remain audit only","testFile":"rust/agent-loop/tests/bug284/cancellation.rs","testLines":"7-40"}` |
| `unlink-coverage` | `{"featureName":"<bare-name>","scenario":"<exact title>","testFile":"rust/agent-loop/tests/bug284/<module>.rs"}` (specific mapping only; preserves scenario) |
| `format` / `validate` / `validate-tags` | `{"file":"spec/features/<name>.feature"}` (explicitly scopes each command) |
| `show-coverage` | `{"featureName":"<bare-name>","format":"json"}` (read only when `output` omitted) |
| `audit-coverage` | `{"featureName":"<bare-name>"}` (file-existence audit only; inspect `exitCode`) |

CLI equivalent example: `fspec link-coverage submission-cancellation --scenario 'Six supervisor inputs canceled before admission remain audit only' --test-file rust/agent-loop/tests/bug284/cancellation.rs --test-lines 7-40`. Other positional forms are declared at `rust/fspec/src/main.rs:1791-1907`; coverage flags at `:2087-2122`.

### Scope hazards and safe application sequence for the native owner

There is **no supported move/clone-feature or scoped generate-coverage command**. `generate-coverage` accepts only `{"dryRun":true|false}` and scans every top-level feature (`generate_coverage.rs:42-48,67-127`). Adding `feature`, `file` or `featureName` will not scope this command. Its dry run also fails to report scenario-set updates to valid existing sidecars. Do not run a live global coverage sweep while other cards are active.

The least lossy route is a native-owner scoped file copy of the original feature and sidecar to the four new filenames, followed by native Fspec pruning. This preserves exact steps, comments, and existing mappings without coverage generation. File copying/retitling is a native filesystem operation, not an Fspec command; if the owner must use Fspec alone, this capability gap is real. Alternative: construct the five features in an isolated staging project, run `generate-coverage` only there, and publish only the ten reviewed feature/sidecar files. Do not change Fspec production code just to do this migration.

1. Snapshot the original feature and sidecar bytes/hashes outside the repository. Check that the four proposed destination basenames do not already exist. Re-read originals before applying; the native owner is active.
2. In the owner’s isolated staging directory, copy the original feature and sidecar to all five intended names. For each copy, retain only its assigned scenarios. Before deleting a mapped unwanted scenario, call scoped `unlink-coverage` for its actual mapped file, then `delete-scenario`; delete unmapped unwanted scenarios directly. `unlink-coverage` recomputes all statistics; `delete-scenario` alone recomputes only total/covered/percent and leaves cached `testFiles`/line counts stale (`delete_scenario.rs:245-301`).
3. Use native exact file edits only where Fspec has no operation (feature title, copying common comment context). Correct the stale approval sentence with `add-architecture`; keep all other acceptance content. Avoid changing story metadata with `set-user-story`: it writes `spec/work-units.json`, not feature Background (`set_user_story.rs:74-79,128-130`).
4. Compare the normalized multiset of `(scenario name, ordered keyword/text steps)` across the five outputs with the original: exactly 14, no omissions/duplicates/weakening. Compare all nine `(scenario,file,lines,implMappings)` records exactly. Verify five unmapped entries remain explicit, one distinct mapped physical file per sidecar, and all tags/context survive. The split must not alter the nine existing test bodies.
5. Format and validate only these five paths; read/audit only these five sidecars. Publish only those ten files after checking the original snapshot is still current. Do not replace shared `spec/work-units.json`, tags registries, or other cards’ files. Update the integration target’s documentation reference to list the split features when the owner handles test metadata; its executable module wiring remains valid.
6. Add meaningful tests for the five missing obligations, with exact `@step` comments beside the relevant assertions. Link each to its real body/range in its assigned physical module; recalculate changed ranges honestly. Run the scoped target under the owner’s established build profile, e.g. `cargo test -p codelet-agent-loop --test bug284_durable_input_admission`, never a workspace-wide test. Preserve actual RED evidence before behavioral implementation.
7. Only then use the ordinary `update-work-unit-status` payload `{"workUnitId":"BUG-284","status":"implementing"}`. No force, temporal skip, task-type change, tag omission, entry-point mapping, concatenated source, or synthetic step-only wrapper.

## Actual status gate: what the split does and does not solve

`update_work_unit_status.rs:175-192` checks incomplete blockers and feature prefill. The Testing→Implementing path calls temporal validation at `:261-264` and step validation at `:268-271`. BUG-284 currently has no `blockedBy`, `virtualHooks`, or explicit `linkedFeatures`.

`update_work_unit_status/step_docstrings.rs:69-138` requires a present, valid sidecar with at least one mapping and exactly one distinct physical mapped file per selected feature. `:142-176` reads the whole physical file and checks **every scenario’s** steps against its comments; it does not follow Rust module declarations or use mapping line ranges for that check. `:196-254` selects top-level features by their first work-unit feature tag. The proposed split fixes the multi-file condition; the five missing tests/steps remain real work.

Full per-scenario mapping completeness is additionally checked on Validating/Done (`update_work_unit_status.rs:273-290,857-894`), and Validating requires implementation mappings (`:896-937`). The current temporal scanner only recognizes `src/**/__tests__/*.test.ts`, not Rust (`:1094-1133`). There is no automatic RED execution/receipt check in this command; absent optional pre-implementing hooks (`:1341-1384`), a successful transition is not proof of execution or acceptance. RED-first and one test per scenario remain repository requirements (`AGENTS.md:203-205,229-231`).

Current review limitation: source and mapping inspection only. No claim that the missing tests exist, that compaction is fixed, that live recovery is verified, or that a transition was attempted. No action was denied or pending; execution belongs to the active native owner.
