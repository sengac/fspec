# Review: RPC-050 — Work-unit context binding (BoardView attach + SessionHeader chip + /detach)

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review-skill)
**Scope:** Single work unit (RPC-050). Parent RPC-030 is an umbrella epic with sibling RPC-* siblings already in `done` — they are out of scope per the user instruction "no scope creep".

---

## Status: ✅ PASS (with minor warnings remediated)

All 11 scenarios across 4 feature files are covered with passing tests. The
implementation matches the architecture notes, the source-shape invariants
hold, both transports round-trip identically, and `cargo build` /
`cargo test --package codelet-fspec-tui` are both green.

---

## Files Reviewed

### Feature files
- `spec/features/work-unit-attach-binding.feature` (3 scenarios)
- `spec/features/slash-command-detach-and-work-unit-binding.feature` (4 scenarios)
- `spec/features/slash-command-detach-cross-transport-parity.feature` (1 scenario)
- `spec/features/slash-command-detach-source-shape.feature` (3 scenarios)

### Tests
- `codelet/fspec-tui/tests/slash_detach_rpc050.rs`
- `codelet/fspec-tui/tests/work_unit_binding_rpc050.rs`
- `codelet/fspec-tui/tests/rpc050_cross_transport_parity.rs`
- `codelet/fspec-tui/tests/source_shape_rpc050.rs`
- `codelet/fspec-tui/tests/common/mod.rs` (MockBackend extensions)

### Implementation
- `codelet/fspec-tui/src/app/dispatch_rpc050.rs` (158 lines, new file)
- `codelet/fspec-tui/src/app/dispatch.rs` (297 lines)
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (288 lines)
- `codelet/fspec-tui/src/app/dispatch_rpc022.rs` (dispatch routing)
- `codelet/fspec-tui/src/components/mod.rs` (Action variants)
- `codelet/fspec-tui/src/store/agent_view.rs` (299 lines)
- `codelet/fspec-tui/src/store/agent_view/work_unit_state.rs` (51 lines, new file)
- `codelet/fspec-tui/src/views/agent.rs` (299 lines)

### Attachments
- `spec/attachments/RPC-050/work-unit-binding.md`
- `spec/attachments/RPC-050/ast-research-work-unit-binding.md`

---

## A. Feature File Compliance — ✅ PASS

- Every scenario has correct Given/When/Then ordering. The
  source-shape scenarios deliberately omit the When step (the
  inspection IS the action), consistent with sibling RPC-*
  source-shape feature files in the project.
- No placeholder text (`[role]`, `[action]`, `[benefit]`) detected.
- Each feature file carries the `@RPC-050` tag and a substantive
  architecture doc string at the top.
- All feature files validate via `fspec validate` (973 features all
  pass; no new violations introduced by this card).

## B. Example Map Alignment — ✅ PASS

- All 10 rules in the example map are covered by at least one
  Gherkin scenario (rules → scenarios traceable chain holds).
- All 9 numbered examples map onto scenarios (cross-checked against
  the linked feature files).
- No open questions (no red cards) on the example map.
- Architecture notes match the actual file layout (`dispatch_rpc050.rs`
  exists, `store/agent_view/work_unit_state.rs` exists, MockBackend
  carries the three documented extensions).

## C. Test Coverage Compliance — ✅ PASS

- Coverage: 100% (11/11 scenarios linked) across all 4 feature files.
- Every Gherkin step has a corresponding `// @step …` comment in the
  test file with text matching the Gherkin step exactly.
- Tests verify real behaviour: backend call counts, store state
  transitions, scrollback chunk text, rendered top-row substrings,
  cross-transport call-counter deltas.

## D. Implementation Quality — ✅ PASS

- **SOLID** — Each `handle_*` helper has a single responsibility.
  Attach / Attached-fold / Detached-fold / SlashDetach are cleanly
  separated.
- **DRY** — No duplicated logic. The spawned-task + action-bus
  round-trip pattern mirrors `dispatch_rpc026::/resume` and
  `dispatch_rpc046::/clear` deliberately and minimally.
- **No shortcuts** — Zero TODO / FIXME / HACK / XXX / todo!() /
  unimplemented!() / panic!() / `.unwrap()` in the new source files.
- **Wired up end-to-end** — `BoardView Enter` → `EnterWorkUnit` →
  `AttachWorkUnitToSession` → `set_work_unit_context(Some)` →
  `WorkUnitAttached` → store fold → SessionHeader chip render.
  `/detach` flow: `SlashCommandSelected(Detach)` →
  `handle_slash_command::Detach` → `handle_slash_detach` →
  `set_work_unit_context(None)` → `WorkUnitDetached` → store clear +
  scrollback reset + TokenState reset.
- **Type safety** — No `any`-equivalent casts; all `Option<…>` and
  `Result<…>` paths handled with `let Some(…) else { return; }`,
  `match`, and explicit `Ok(()) / Err(e)` arms.
- **Error handling** — Backend `Err` is routed via
  `Action::EmitSessionNotice` to the originating session (preserves
  local state per rule [6]).
- **File-size invariant** — Pinned by the source-shape scenarios and
  verified by `source_shape_rpc050.rs`. Largest touched files:
  `dispatch.rs` = 297, `dispatch_rpc020.rs` = 288, `agent_view.rs`
  = 299, `agent.rs` = 299. All `< 300`.
- **Import style** — Standard Rust `use` statements; no shortcuts.

## E. Build & Test Verification — ✅ PASS

- `cargo build --package codelet-fspec-tui` → clean (no warnings on
  RPC-050 files).
- `cargo test --package codelet-fspec-tui` → all tests pass across
  the entire package, including the four RPC-050 test files (1 + 4
  + 3 + 3 = 11 scenarios green).
- The dependency-rule regression (`no_codelet_napi_*`) still holds.

## F. Cross-Cutting Concerns — ✅ PASS

- No security concerns (no untrusted input, no secrets exposed).
- No performance concerns (one tokio task spawn per attach/detach;
  no unbounded loops; `lookup_work_unit_context` walks
  `COLUMN_ORDER` which is bounded).
- Cross-transport parity is explicitly tested
  (`rpc050_cross_transport_parity.rs`) and the stub call-counter
  arithmetic matches rule [7].

---

## 🔴 Critical Issues

None.

## 🟡 Warnings (Fixed during this review)

1. **Coverage line-range drift** — All 11 linked test ranges were
   slightly off (typically missing the trailing `@step` block or
   overshooting into adjacent functions). Each scenario was
   `unlink`-then-`link`-ed with the precise function-span line
   range. After the fix:
   - `slash_detach_rpc050.rs` → scenarios pin lines `126-218`,
     `224-249`, `256-283`, `290-320` respectively.
   - `work_unit_binding_rpc050.rs` → `80-124`, `131-153`, `169-207`.
   - `source_shape_rpc050.rs` → `84-136`, `139-162`, `165-194`.
   - `rpc050_cross_transport_parity.rs` → `70-138`.
2. **Stale docstring** —
   `codelet/fspec-tui/src/app/dispatch_rpc022.rs:209` claimed
   *“Route the seven RPC-022 Action variants…”* but the helper now
   handles 8 RPC-022 arms plus 3 RPC-050 arms (Attach / Attached /
   Detached). Updated to *“Route the RPC-022 (model / thinking /
   role) and RPC-050 (work-unit attach / detach) Action variants…”*.

## 🟢 Observations (Out of scope / informational)

1. The work-unit metadata note `architectureNotes[0]` says “Add **two**
   new Actions” but then enumerates three. Cosmetic typo in stored
   metadata only — not a behavioural defect; left as-is to avoid
   churning historical record.
2. Tag `@cross-transport` is registered as `Unknown` in the tag
   registry. The tag is used across **5** existing RPC features
   (`rpc018-`, `rpc020-`, `rpc022-`, `rpc026-`, and this card). This
   is a project-wide registry gap, not an RPC-050-specific issue,
   and is out of scope for this card per the user’s “no scope creep”
   instruction.
3. The BoardView `EnterWorkUnit` arm + `SessionCreated` arm both
   dispatch `AttachWorkUnitToSession` when `current_work_unit_id` is
   set. This is intentional double-fire to handle the
   lazy-session-creation flow (rule [0] + architecture note 2). The
   first dispatch silent-no-ops with no session; the second after
   `SessionCreated` performs the backend round-trip. The behaviour
   matches the architecture notes; no concern.

---

## Coverage Verification (Post-Fix)

| Feature file                                                  | Scenarios | Coverage |
|---------------------------------------------------------------|-----------|----------|
| `work-unit-attach-binding.feature`                            | 3         | 100%     |
| `slash-command-detach-and-work-unit-binding.feature`          | 4         | 100%     |
| `slash-command-detach-cross-transport-parity.feature`         | 1         | 100%     |
| `slash-command-detach-source-shape.feature`                   | 3         | 100%     |
| **Total**                                                     | **11**    | **100%** |

---

## Fix Results

### RPC-050: Work-unit context binding
- 🟡 Coverage line-range drift across all 11 linked scenarios →
  ✅ Fixed: every scenario unlink+relink to exact `fn` line span.
- 🟡 Stale "seven RPC-022 Action variants" docstring in
  `dispatch_rpc022.rs:209` →
  ✅ Fixed: docstring rewritten to describe both RPC-022 and
  RPC-050 routing.

## Final Verification

- All tests pass (cargo test --package codelet-fspec-tui): ✅
- Build succeeds (cargo build --package codelet-fspec-tui): ✅
- Coverage complete (11/11 scenarios linked): ✅
- Feature files valid (fspec validate): ✅
- Source-shape invariants hold (all touched files < 300 LoC): ✅
