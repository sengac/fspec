# Epic Review: provider-settings-parity — Model/Provider View Fixes (PROV-101..104)

**Date:** 2026-06-21
**Reviewer:** Claude Code supervisor + spawned ACDD review worker (session 741b5bb9)
**Work Units:** PROV-101 (story), PROV-102, PROV-103, PROV-104 (bugs)

## Summary
- 🔴 Critical: 0 outstanding (all fixed)
- 🟡 Warnings: all addressed or rolled forward (see per-card notes)
- 🟢 Observations: documented TS-parity gaps (no real OAuth/profile-edit flows yet in Rust frontend)

## Work Unit Results

### PROV-101 — Remove all selection fallbacks — ✅ PASS (after fixes)
Removed default-to-anthropic in create_session/create_isolated_session; deleted
detect_default_provider Claude-first priority chain → resolve_unambiguous_provider (0→Err, 1→Ok,
>1→Err); removed model-selector first_selectable_or_zero auto-snap; deleted dead fallback_models.json.
Review WARNs fixed: empty-SessionId decline now surfaced explicitly via SessionCreationDeclined →
ErrorDialog (no silent swallow); dead RPC-022 ModelSelectorDialog (with its index-0 fallback) deleted;
touched code extracted (provider_resolution.rs, session_creation.rs; dispatch.rs 291 LoC).

### PROV-104 — Model view scroll/viewport parity — ✅ PASS
render_body no longer steals content rows for inline ↑/↓ glyphs; full-window slice + dedicated
scrollbar column (TS parity); page_up/page_down added. New scroll_tests.rs renders to a real
TestBackend and asserts the SELECTED row's ▸ glyph is actually painted at top/bottom/mid/End/PageDown.
This is the user's #1 "doesn't scroll" complaint — fixed and proven. Reviewer's test-allow FAIL
adjudicated as project-wide convention (1011 files) — not a defect.

### PROV-103 — Up/Down + Enter dead in nav tree — ✅ PASS
move_clamped/adjust_scroll now bound by nav_len() (nav_items.len() when populated, else legacy
provider count) → cursor reaches child rows, scroll follows. TS parity: plain ±1 clamp, no
skip-logic (this tree has no non-selectable headers). mod.rs 299 LoC. WARN (Enter/d index-space
fallthrough on child rows) rolled into PROV-102.

### PROV-102 — Enter on OpenAI profile opens Anthropic — ✅ PASS
Index-space mismatch ELIMINATED. Enter + d dispatch by focused_nav_item().kind + its own
provider_id (Profile/AddProfile/OAuthLogin/OAuthStatus); visible_providers()[selected_index]
fallthrough reachable only when nav_items empty (legacy). Proof test: multi-provider tree, Enter on
OpenAI profile asserts provider_id=="openai" (not anthropic@idx1). All files <300 (list.rs 265,
list_actions.rs 130, nav_tree_ops.rs 92, mod.rs 296).

## Final Verification (supervisor, after disk cleanup of 24G regenerable incremental cache)
- PROV-101: providers 3/3, sessions 3/3, decline 3/3 — all pass
- PROV-102: prov102_nav_item_action_dispatch 7/7 pass
- PROV-103: prov103_nav_tree_navigation 5/5 pass
- PROV-104: model_selector::scroll_tests 6/6 pass
- clippy -p codelet-fspec-tui --all-targets -- -D warnings: clean
- All 7 feature files @done with component + feature-group tags; coverage 100% per reviewer audits
- fallback_models.json confirmed deleted; no remaining populated-tree visible_providers fallthrough

## Recommended follow-up cards (out of scope here)
- Wire real Rust-frontend flows: profile create/edit, OAuth login, OAuth disconnect, per-profile delete
  (currently honest no-ops / OAuthNotice placeholder, each carrying correct provider_id).
- Dedicated refactor cards for pre-existing 300-line megafiles: model_selector/mod.rs (2124),
  handle_impl.rs (1766), manager.rs (2805), rows.rs (819).
- Pre-existing session_manager_shape.rs brittle source-layout tests (make whitespace-insensitive).
