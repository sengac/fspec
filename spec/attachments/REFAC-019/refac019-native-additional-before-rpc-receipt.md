# REFAC-019 additional before-rpc mapping receipt

FINAL SAFE STOP: User selected Prepare Tooling Proposal. Latest read-only
recheck confirms the 9 final sidecar readbacks below unchanged, all plan source
fingerprints and both Mux fingerprints unchanged, and 22/22 protected hashes
matching. Held row 16 remains exactly unchanged. No further source/metadata
writes, tooling activation, daemon restart or canonical Done transition made.
Parent owns the filtered checkpoint-pair rerun, settled audit, proposal
attachment and accurate pending state. No new work is claimed by this receipt.

Final with one native gate held. Own additional rows 0–16 of
/tmp/refac019-closure-current-full.json only. Source-provenance reconstruction
uses unchanged /tmp/refac019_closure_audit_support.py load_plans/relocate helpers:
each referenced original HEAD line is checked against its physical target;
only plan-documented blank separators may disappear. No guessed offsets.
All plan source fingerprints and 22 protected hashes match before writes.
No owned sidecar is protected. No production/test/feature/status edits authorized
for this mapping-only phase. BUG181's separately approved outline correction is
now complete; final evidence is /tmp/refac019-bug181-lineage-correction.md.

## Final operations and readback

- Additional rows 0–15 completed: 16 scenario mapping repairs across 9 sidecars.
  Fifteen original implementation entries become 16 exact physical entries.
  482 original reference occurrences become 481: only original embedded.rs
  blank separator 69 disappears in row 3 (original 1-87 → root 1-68,82-83 plus
  sessions.rs 5-20). No nonblank reference or duplicate occurrence was dropped.
- Three root test ranges corrected to exact complete audited function ranges:
  source_shape_trait.rs 95-131, 135-169, 173-235. Each function identity and
  closing brace verified. Other test ranges preserved, including inherited
  partial ranges not authorized for semantic repair.
- Native calls: 17 successful link-coverage, 13 successful unlink-coverage,
  1 rejected link-coverage. No rejected request repeated. No staged copies remain.
- Final complete-content hashes match all 8 unaffected expected feature hashes.
  The ninth, mux-surface, is 98a45161f5d8629f4ab4ed4b2028533ac1ee80996d8bb3d53ce502172fcecc27.
  Its ONLY difference from the pinned full-repair expectation is unchanged
  blocked row 16. Applying that single expected array replacement in memory
  reproduces pinned hash 82d9f3c8b5fde8b8474d15e20019e8f369b954482e99263055c163731d1c43e3
  exactly. All unrelated arrays, scenario names and multiplicities preserved.
- Final source fingerprints, both Mux hashes and all 22 protected hashes still
  match. No unstable inputs during readback. No source/spec/test edits, builds,
  commits, status resets, skips or global coverage generation performed.

After BUG181 completion, these same 9 sidecar readbacks were rechecked unchanged
and all plan/Mux/protected production fingerprints still match. Only additional
row 16 below is held; BUG181 is no longer blocked. The approved BUG181 changes
are feature/required @step comments only, outside this mapping-only phase.

## Native gate: additional row 16 (left unchanged)

Feature mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick,
scenario "Clicking outside the dropdown closes it while the click still lands
on the pane". Native link to navigator_mux_routing.rs:17-37 rejected:
"Given the mux 'Actions' dropdown is open" has no matching test step.

Investigation: feature line 205 still says Actions, whereas the genuine existing
test at menu004_mux_surface.rs:1341-1392 says Kanban and actually opens the
first menu, renders the dropdown, clicks the Files pane, and asserts closure,
Files focus and cleared bar focus. No missing proof was fabricated.
The old mapping's 1208-1252 range is also a historical semantic misassociation:
it spans the clicking-a-chip test, not the outside-dropdown test. The helper's
required_functions entry inherited that same incorrect old range; it is not
safe evidence to move this scenario onto the chip function.

Readback preserves old row exactly:
test rust/fspec-tui/tests/menu004_mux_surface.rs:1208-1252 →
impl rust/fspec-tui/src/views/navigator_mux_events.rs:[36..56].
Provenance target remains navigator_mux_routing.rs:[17..37]. Parent coordination
is needed for the legitimate Actions→Kanban feature-step correction and actual
outside-dropdown function range before a materially corrected native request.
This worker made neither semantic edit and did not retry the rejected request.

## Expected canonical scenario-content hashes pinned before writes

The complete scenario/test/implementation content, including multiplicity and
all unrelated arrays, is retained; only test/impl collection order is normalized.
Rows 4/5/6 also require exact source-proven root-function ranges 95-131,
135-169 and 173-235 respectively. All other existing test ranges stay unchanged.

```text
dual-transport-parity 6041a5837be994036cb13dfc298c9e982ed03937c0fc8739b605b2492945f278
filter-resume-session-list-by-current-project 251801fc73a46ccf5b161cc917fad7adcf762fcbd63107dd43abc9f41cf00ded
fspec-tui-app-bootstrap-rpc009 f181a8c9ad857134adff5615b7ab282f0a9db63d4b80c3771319e19618655bb5
fspec-tui-embedded-backend f574a10d6e3fd0d297aeb12b643ea3f9d4587e98c0f6d05be60fc43f11c29034
fspec-tui-trait-surface 667d649076015b6bc2a4ec16e0657da2e3da35b1a99a0c32b0c00b37841f0b3b
menu-bar-click-away-de-select-and-no-stale-chip-highlight 6ac646dbca13c7507995a84b87c7cc3bf53505424d7dce301c77e6b150dd33c8
menu-dropdown-row-click-must-execute-the-entry-click-away-must-close-de-select-the-bar 49cd1de9f89fd0dd440e2a201f4a2b423c7e278fc21432e5d7e96cd4abb78b82
mux-agent-input-paste-drop-routing e5e7c1cda2719942a5256fdec3d3378cbb17de8b239de10db1ead866cf7e3436
mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick 82d9f3c8b5fde8b8474d15e20019e8f369b954482e99263055c163731d1c43e3
```

Mux initial source hashes (all owned references independently text-verified):
- navigator_mux_events.rs: 4d5ed229e538bf22cdbd5f701e7e305170780045bb0e9cce2345b935d37ed77f.
- navigator_mux_routing.rs: 172d038d5bf9ae69fed869a4c950bff4d32a089079094fdf26767d87f73fcc53.
