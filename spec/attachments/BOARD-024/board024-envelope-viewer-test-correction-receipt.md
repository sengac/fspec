# BOARD-024 tests-only envelope and viewer correction

## Ownership and execution boundary

Only existing board024/canonical.rs, support.rs, loading.rs, browser.rs and
attachments.rs were changed in this assignment. No production, feature,
coverage, dependency, protected BUG-283, dispatch, or worker2356-owned additive
API/canonical backend target was edited. Parent owns builds and metadata.

No Cargo build or test was run here. Prior typed-completion compiler RED remains
historical evidence; the new seam references also require parent implementation
and execution before any GREEN claim. Scoped rustfmt --check and git diff --check
pass. All eight target sources remain below 300 lines; the largest is the
unchanged details.rs at 287. Changed lengths: attachments 201, browser 56,
canonical 103, loading 266, support 283.

## Canonical result expectations

- SharedFspecService is explicitly attached with with_cwd(root.path().to_path_buf())
  before either embedded or WebSocket backend construction.
- Keep Action::BoardCardDetailsLoaded { request_generation, work_unit_id, result }
  exactly as staged. The result remains Result<String, String>.
- Success String is decoded as the accepted {id,workspace,workUnit} envelope.
  Held completions require the exact outer ID, absolute workspace, object record,
  and the expected record ID for these fixtures.
- The complete canonical scenario compares workspace to the fixture's canonical
  root and workUnit to the full original JSON Value, independently of UI rendering.
  Both real transports run this proof; the client cwd must remain distinct.
- Existing rich/unknown/legacy fields, Unicode/rendering assertions, typed result
  cardinality, distinct generations, delayed success/error rejection, captured
  card, selection, chooser and Board/Mux restoration assertions remain intact.

## Exact proposed App controller seams

These are tests-first expectations, not production edits or claims of an
already implemented API:

```rust
pub fn set_board_attachment_launcher_for_test(
    &mut self,
    launcher: std::sync::Arc<
        dyn Fn(&str) -> Result<(), String> + Send + Sync
    >,
);

pub fn set_board_attachment_viewer_enabled_for_test(&mut self, enabled: bool);
```

Forward both exclusively into the feature-owned card attachment controller.
The default viewer setting is enabled. The normal enabled path must own and
retain a real viewer lease rooted at the canonical envelope.workspace, rather
than reusing the legacy App viewer rooted at the client's cwd. Disabled means
deterministic viewer-unavailable failure without invoking the launcher. Production
normally supplies its real browser launcher; tests supply a local recorder
returning Ok or the deterministic error below. The same production feature path
must invoke the callback and deliver failures visibly through App.

The proposal was attempted via AgentManager message to parent2701b174 before
test edits, but returned session_busy and was NOT queued. It was then stated in
the session's normal commentary before staging. Parent must review the precise
seam above before production and coordinate any spelling/signature change; no
approval response was observed here. No busy-message retry or waiting on the
supervising parent was performed.

## Attachment proof

- Removed the Linux PATH/xdg-open/environment shim entirely. Browser fixture is
  an Arc<Mutex<Vec<String>>> recorder, with no browser invocation or global state.
- Success, unavailable viewer, and injected browser failure each run through both
  embedded and WebSocket-backed canonical fixtures.
- Canonical attachment files with duplicate basenames have distinct exact
  contents incorporating their full original paths. Unicode, spaces, hash and
  percent characters remain unchanged in Action::OpenAttachment.
- Actual recorded URLs must be HTTP localhost URLs on their real viewer port,
  exactly equal to App::attachment_url(port, original_path). There is no fake
  port in activation setup. The existing pure URL-encoding assertion at port
  8123 remains only as a standalone encoding contract, not a server fixture.
- Reqwest GET of the actual recorded URL must return 200 and the complete exact
  viewer-template rendering of that specific canonical file's contents. This
  proves serving the right rooted file, not merely recording a plausible URL.
- Launcher success records one URL per activation. Repeated activation records
  one more identical URL without closing details or losing selection/context.
- Injected launcher failure is exactly
  "BOARD024 injected browser launch failure" and must appear, with the original
  path, in the actual visible Error region. The still-live correctly rooted
  viewer must also serve the expected file even when browser launch fails.
- Viewer-unavailable activation makes zero launcher calls; missing files,
  directories and lexically rejected paths likewise must not call the launcher.
  Existing precise categories, paths, layer IDs and usability assertions remain.
- No symlink containment or new path security policy was introduced. Literal
  percent-encoded-looking filename support was not newly claimed by these tests.

## Parent next steps

Review and implement the two feature-controller seams with the agreed real viewer
lease, after the appropriate causal test boundary. Run the corrected original
board024_enter_chooser target and worker2356's disjoint targets. Refresh coverage
ranges only through coordinated native operations; this worker made no metadata
writes. These edits do not establish feature completion or canonical Done.

## Tooling-proposal pause: final obligations and mapping locations

Parent subsequently directed Prepare Tooling Proposal and a stop after these
already authorized test corrections. No further tests or production assumptions
were added. No tooling was activated and no daemon was restarted. REFAC metadata
verification remains worker2032-owned; this worker makes no further metadata edits.

Current compile/test obligations, not newly executed evidence:

1. Preserve the already captured causal compile RED for the direct typed
   BoardCardDetailsLoaded variant. Do not replace typed pattern matching with
   arbitrary queued actions to compile around the absent contract.
2. The two proposed App launcher/viewer seams are staged test references only;
   no parent approval or production implementation is claimed. Until supplied,
   they are additional expected compile blockers, not observed new compiler RED.
3. After authorized production resumes, parent must compile/run the corrected
   target, then exercise every deeper assertion across both transports. No new
   compile result, viewer HTTP success or behavioral GREEN was observed here.

Exact coverage sidecar (unchanged by this correction assignment):
spec/features/board-card-enter-chooser-and-complete-details.feature.coverage.
Its existing ranges must be checked/refreshed through coordinated native tooling
later; they must not be assumed current following these source edits.

Current physical test ranges requiring comparison with that sidecar:

| Source | Range including attributes | Test |
| --- | --- | --- |
| rust/fspec-tui/tests/board024/attachments.rs | 35-131 | attachments_retain_exact_paths_and_unavailable_viewer_failures_retain_details |
| rust/fspec-tui/tests/board024/attachments.rs | 133-166 | missing_directory_and_rejected_attachment_paths_show_contextual_errors |
| rust/fspec-tui/tests/board024/attachments.rs | 168-201 | display_sanitization_preserves_original_identifiers_and_attachment_actions |
| rust/fspec-tui/tests/board024/loading.rs | 8-129 | full_canonical_details_agree_across_embedded_and_websocket_backends |
| rust/fspec-tui/tests/board024/loading.rs | 131-174 | missing_and_malformed_canonical_cards_show_errors_without_losing_the_chooser |
| rust/fspec-tui/tests/board024/loading.rs | 176-227 | reopening_the_same_card_rejects_stale_success_and_error_completions |
| rust/fspec-tui/tests/board024/loading.rs | 229-266 | dismissed_requests_cannot_resurrect_dialogs_or_replace_another_card |

Other target tests were not changed in this correction assignment. Earlier BOARD
correction history remains in /tmp/board024-corrected-test-receipt.md; its old
ranges are historical, not replacements for the current table. The completed
delegated REFAC metadata receipt is
/tmp/refac019-native-mapping-rpc022-through-rpc061-receipt.md, including its retained
RPC026 blocker. This receipt is the final BOARD tests-only handoff. Worker stops
without more production, metadata, build, activation or completion actions.
