# REFAC-019 semantic gate correction receipt

LATEST STATE: parent GREEN has now been read and both RPC026 mapping replacements
are complete through native commands. The initial pending-GREEN sections below
record the earlier handoff; the final continuation section supersedes them.

## Authority and boundary

Read /tmp/refac019-approved-coverage-continuation.md first and wrote
/tmp/refac019-semantic-plan.md before any test/spec/mapping edits. This worker
owns only RPC026 deletion-parity accuracy, RPC050 BUG205 criterion reconciliation
and Mux outside-dropdown lineage, including their three named feature sidecars.
No production, core tooling, CLI, BOARD, BUG284, histories, security, status,
install, restart, build, commit or push operation was performed.

Original feature/sidecar bytes and the RPC026 test are retained in
/tmp/refac019-semantic-before/. No original lineage artifact was overwritten.

## RPC026: stronger executable proof authored; parent execution pending

Replaced the defective deletion test, preserving its original root-qualified
name embedded_and_websocket_delete_session_byte_identical. It now:

- Constructs two real independent SharedFspecServices with separate real
  SessionManagers and watchers. Their logical workspace is identical; physical
  persistence roots are distinct embedded/ and websocket/ directories.
- Runs experiments sequentially under the existing process-global data-directory
  mutex, resetting the session-store singleton at each fixture boundary. This
  respects actual process-global persistence rather than claiming concurrent
  per-service store isolation that production does not provide.
- Seeds three valid complete SessionManifests with identical fixed UUIDs,
  timestamps, names, provider/model and project. Fixtures serialize complete
  manifests directly before real SessionStore discovery: save_session would
  rewrite updated_at and make the intended identical timestamp fixture false.
- Requires exact nonempty initial SessionInfo lists, identical serialized initial
  lists and independently seeded survivor manifest bytes. Missing manager/empty
  fallback or invalid manifest fixtures therefore fail before deletion.
- Deletes the same middle UUID through each actual backend. Requires exact
  ordered surviving IDs [s1,s3], full unmodified survivor SessionInfo objects and
  byte-identical serialized remaining SessionInfo lists. Both roots must lose
  the middle manifest while retaining both controls. No returned list is discarded.

The two original list/history tests and core-deletion test remain byte-exact.
Only the complete strengthened deletion test moved to an ordinary root include;
independent seeding/support is in its fixture module. The existing temp setup now
uses the shared codelet_test_helpers::setup_temp_directory helper.

Maintained Rust files / physical lines / SHA256:

- rust/fspec-tui/tests/rpc026_cross_transport_parity.rs: 180;
  3f060d2afaa54902ae2e9832ac41e473a8bfc703f10a05b96f194219818b2068
- rust/fspec-tui/tests/rpc026_cross_transport_parity/deletion_parity.rs: 122;
  7e31b98ffdf6a0728ae34f4c7cb230a02b3b8b9ca31d2982fd86f02efc9e8029
- rust/fspec-tui/tests/rpc026_cross_transport_parity/fixture.rs: 95;
  8c8f8eb11225f8e700ec091acb74c206ba128ee23224eb491b7d4ea1b3462b1b

RPC026 coverage sidecar remains wholly unchanged, including both historical
1-300 test ranges and the old lib.rs:1-700 implementation occurrence. Execution
is a real gate, not presumed GREEN. After parent semantic proof, the deletion
mapping must target the actual included test and preserve the entire five-file
original implementation union, not just a convenient deletion method:

- rust/rpc/src/lib.rs:1-62
- rust/rpc/src/service_declaration/finish.rs:3-8
- rust/rpc/src/service_declaration/workspace.rs:5-171
- rust/rpc/src/service_declaration/providers_sessions.rs:5-224
- rust/rpc/src/service_declaration/supervision_credentials.rs:5-247

The unchanged core test is now physically root133-180. Its historical broad
mapping and inherited core implementation range need an explicit disposition;
they were not silently narrowed or claimed corrected during deletion work.

## RPC050: authoritative BUG205 negative guard preserved

Verified completed remove-detach-slash-command-with-attach-path-intact.feature,
actual Action domain sources, attach-only dispatch_work_unit_binding.rs and
the existing positive attach/negative detach assertions in source_shape_rpc050.rs.
No production or source-shape test was edited.

Three native update-step operations changed only the obsolete positive criteria:
WorkUnitDetached, handle_work_unit_detached and handle_slash_detach now explicitly
must NOT exist after BUG205. Historical scenario names, the original RPC050
supersession note and all attach requirements remain. The historical introduction
and business-rule comments remain historical; no removal of old evidence.

After actual semantic proof, native link-coverage appended the complete guarded
Action test142-167 with the exact original physical provenance replacement
action_agent_navigation.rs:106-124. Readback verified the complete replacement
and retained first original row before native unlink-coverage removed that
obsolete row. Final readback has exactly one replacement occurrence.

Important honesty detail: the original mod.rs:414-432 range is an inherited
scenario/implementation association covering resume/attach-to-session declarations,
not the actual RPC050 work-unit variants. Its successor106-124 was independently
compared against HEAD, indentation-normalized, and is exact. It was NOT swapped
for a different scenario-named range; current attach assertions inspect the
entire real composed Action item (actual work-unit variants are in
action_session_context.rs). Every original covered line is retained.

Other two RPC050 mapping records remain byte-equivalent as JSON, including
inherited stale dispatch ranges reaching158 although the current file has127
lines. These pre-existing discrepancies were not silently shortened or repaired.

## Mux: criterion and actual outside-dropdown test aligned

Verified the existing menu004_mux_surface.rs:1341-1392 test opens the first
Kanban dropdown, renders its geometry, clicks the Files pane and asserts ALL:
dropdown closed, Files pane focused and bar focus cleared. The unrelated old
1208-1252 chip-test range was not used as outside-click proof. The oversized
menu004 test source was not edited.

Native update-step changed only this scenario's Given from Actions to Kanban.
Native link-coverage appended actual test1341-1392 with exact routing17-37;
readback confirmed original and full replacement before native unlink-coverage
removed only the first old row. Final readback has one actual outside-click row.
Old navigator_mux_events.rs:36-56 and new navigator_mux_routing.rs:17-37 match
line-for-line after indentation normalization. All three actual assertions remain.

## Validation and native-operation outcomes

- Native Gherkin syntax validation: all three features pass.
- Native scoped tags: RPC050 and Mux pass. RPC026 fails for inherited unregistered
  @core and @cross-transport tags. Neither feature nor tags registry was changed
  to hide that result; it requires parent disposition.
- rustfmt --check passed for all three touched Rust sources; scoped git diff
  --check passed. Every touched maintained Rust source is below300 physical lines.
- JSON readback confirms all unrelated scenarios/mapping multiplicity and unknown
  non-stat top-level fields in the three owned sidecars remain unchanged. RPC026
  sidecar is wholly unchanged. Only the one Action mapping and one Mux mapping
  were replaced; native commands recomputed their stats.
- First update-step attempt was rejected before mutation because the live native
  schema requires currentStep (repository source showed current_step). Corrected
  argument spelling succeeded. No rejected coverage link was retried unchanged,
  no validation skip or global/scoped metadata generator was invoked.
- Parent notification attempts returned session_busy and were NOT queued. This
  receipt/final response is the handoff; no build was started by this worker.

## Parent-owned next execution

From rust/ with the parent's shared Cargo scheduling:

```sh
CARGO_PROFILE_CI_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 cargo test --offline --profile ci-test -p codelet-fspec-tui --test rpc026_cross_transport_parity embedded_and_websocket_delete_session_byte_identical -- --exact
```

Then the complete rpc026_cross_transport_parity target (all four original names),
and focused unchanged guards if needed:

```sh
CARGO_PROFILE_CI_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 cargo test --offline --profile ci-test -p codelet-fspec-tui --test source_shape_rpc050 action_enum_declares_rpc050_variants -- --exact
CARGO_PROFILE_CI_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 cargo test --offline --profile ci-test -p codelet-fspec-tui --test source_shape_rpc050 dispatch_work_unit_binding_declares_attach_helpers_and_no_detach_helpers -- --exact
CARGO_PROFILE_CI_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 cargo test --offline --profile ci-test -p codelet-fspec-tui --test menu004_mux_surface scenario_clicking_outside_the_dropdown_closes_it_while_the_click_still_lands_on_the_pane -- --exact
```

No compilation, runtime success, final six-root closure, native activation or
canonical Done is claimed. RPC026 provenance replacement remains pending parent
test execution, and inherited tag/metadata discrepancies remain explicit.

## Final continuation: parent GREEN and verified native RPC026 mapping completion

Parent explicitly authorized mapping continuation after
/tmp/refac019-semantic-and-selection-validation.log reported rpc026 target4/4,
including the strengthened deletion test, plus unrelated parent-owned BOARD
selection helper8/8. Cargo exit0. Log SHA256:
96add30dd04bdf01e7fb0c7f1349369dd647d7122106da9f22571bb34909ecba.
This worker read the receipt and ran no build. All three test-source SHA256 pins
above still match the GREEN inputs; all five production source pins match the
original RPC provenance plan. No source changed in this continuation.

### Deletion mapping: actual child and complete five-file provenance union

Native combined link appended the actual deletion_parity.rs:1-122 test with
lib.rs:1-62. Four native implementation-only links completed the full five-file
union listed above, selecting the new distinct child file unambiguously.
Readback verified the old row was byte-equivalent as JSON and the new child
carried all five exact arrays, in order, before native unlink removed only the
old root test occurrence. Final deletion occurrence count1; no original row
was discarded before its complete successor existed.

Old lib.rs1-700 becomes698 retained physical references; old blank separators236
and457 are the only omitted references, as explicitly recorded in the frozen
plan. These are not five scenario-named substitutes or a narrowed method mapping.

### Core deletion mapping: explicitly approved blanket-span reassociation

Parent separately and explicitly approved replacing the historical broad test
span1-300 with the actual unchanged core deletion test root133-180. This is a
test-accuracy reassociation, NOT a claim that all old300 referenced test lines
physically relocated into this one function. The entire original feature sidecar,
including both original complete rows and arrays, is archived at
/tmp/refac019-semantic-before/rpc026-cross-transport-parity.feature.coverage,
SHA2566138af0c52c3504f6c34410cee10d7576593a6476eba63e8ea93612c884f9eaa.
The entire original246-line test is also archived there under its original
basename, SHA256ac3d5d6f9b80cd686f1da7f95f2e2593ebedb706f2a06729b24194f7ef21bc78.
Thus the original broad span is preserved as historical evidence, including its
pre-existing54 out-of-file references247-300, not rewritten into fabricated code.

Native combined link appended root133-180 with the EXACT live implementation
object sessions.rs1-100. Readback verified old and new occurrences were both
present, that the replacement range covered the entire unchanged test, and that
the full implementation object equaled the original before native first-match
unlink removed the first obsolete root occurrence. Final core occurrence1;
implementation object unchanged. Canonical compact sorted-key JSON SHA256 of its
implMappings array:
f093df0ca0e3edc0ec1196c18f623318a3c6b79e9f34f17999f693be75fc16d0.

All three other original test blocks remain byte-identical, including their
scenario docs and assertion bodies, and all four target tests ran GREEN:

- list_sessions block:
  0dc5456185a85ad83d2ac2c7387b17cb94d68c9596238160f4c1cb714a356fa7
- persistence_search_history block:
  524934d4bc30a89c1f4739b0a5ab8edef795ca2af648bd3af8182df1a85ce778
- core_delete_session block (now133-180):
  2e57533551972c9574ed52ea2a4748333ed91eb19b9b045f4dccfaa2c5da7266

The list/search tests remain unmapped under this two-scenario feature as before;
no extra scenario rows or invented lineage were added.

### Final scoped native/readback audit

Native show-coverage confirms RPC026 has2/2 mapped scenarios, two test files and
six implementation files; RPC050 has3/3 mapped scenarios. These are native
metadata summaries, not proof that all historical ranges are valid. Final exact
JSON comparison also verifies:

- RPC026: two scenarios, two test occurrences, one core implementation plus the
  full five-file deletion union; all unknown non-stat top-level fields retained.
- RPC050: three scenarios/three test occurrences; only the approved Action row
  changed. Other two rows retain their original objects exactly.
- Mux:32 scenarios/33 test occurrences; only the approved outside-dropdown row
  changed. All other records, including duplicate multiplicity, remain exact.
- Scoped git diff --check passes. No additional specs/source/status changes.

Final sidecar SHA256 values:

- RPC026:64edae170d2253408d691a9520ed294ffe3b465fd851e01fb35f45755badf7ff
- RPC050:25c8868746c6f416732d675337e43ad0ec6324638d0e05e1bd48112aa205d8ce
- Mux:1479aba77380c130dec3bc3f6f8f1e4fcf6522890381bc1e95b4546f2f69c43f

### Unchanged historical baseline issues, not new mapping regressions

- RPC026 core implementation sessions.rs1-100 exceeds the actual27-line source.
  Parent explicitly required preservation of this exact inherited array, not an
  unrelated core implementation reassociation. It is NOT silently fixed or
  represented as current full physical implementation coverage.
- RPC026 @core/@cross-transport remain inherited unregistered tags. Feature
  syntax passes; strict scoped tag validation does not. No tags changed.
- RPC050 untouched dispatch arrays1-158 and21-158 exceed current127-line source;
  untouched helper test165-194 is the inherited historical span, not the entire
  current helper guard test171-199. No unrelated row was rewritten.
- RPC050 Action implementation preserves the original physical association,
  even though it does not name the work-unit attach variants; the complete
  composed-source assertions remain intact.

Exact scoped semantic audit successors and fixed source prerequisites are
provided in /tmp/refac019-semantic-audit-overrides.json. These are declarative
handoff evidence for the parent-owned settled audit, not a changed audit helper,
new baseline, blanket assertion whitelist or permissive hash normalizer. No
global settled audit success or canonical Done is claimed by this worker.
