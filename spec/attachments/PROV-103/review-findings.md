# Review Findings — PROV-103 (Up/Down and Enter dead in provider settings nav tree)

**Reviewer:** spawned ACDD reviewer (session 741b5bb9), supervised.
**Result:** PASS (with WARN). No 🔴.

## Verified
- `nav_len()` (`nav_tree_ops.rs:67-73`) = `nav_items.len()` when populated, else `visible_providers().len()`.
- Both `move_clamped` (`mod.rs:251-261`) and `adjust_scroll` (`mod.rs:241-249`) bound by `nav_len()`.
- Live key path proven reachable: navigator_events.rs:28 → mod.rs:223 List arm → list::handle_list_key
  → list.rs:70/80 move_clamped(±1) → nav_len(). Not test-only.
- PROV-101 mandate honored: total==0 → clamped no-op (mod.rs:253-255); length fallback is not a
  selection fallback; no implicit row-0 snap (confirmed by empty_nav_tree test).
- TS parity confirmed: src/tui/inputHandlers/listModeHandler.ts:69-89 clamps Up/Down by
  navItems.length-1, plain ±1, NO skip-non-selectable loop — matches the Rust plain clamp. The
  "no non-selectable header rows in this tree" claim is accurate.
- mod.rs = 299 lines (under 300). Gates all green (cargo test full crate 0 failed; clippy
  --all-targets -D warnings clean; fmt --check clean; build success). Coverage 100% (5/5), audit 10/10.
- // @step byte-exact; tests genuine offline behavioral tests.

## 🟡 WARN → ROLLED INTO PROV-102 (must fix)
**Enter (and `d`) legacy fallthrough uses a mismatched index space — now triggerable.**
`list.rs:117-135` (Enter) and `list.rs:137-153` (`d`): NavItemKinds other than Provider/ApiKey
(i.e. **Profile, AddProfile, OAuthLogin, OAuthStatus**) fall through to
`view.visible_providers().get(view.selected_index)`. After PROV-103, `selected_index` is a
`nav_items` index, not a `visible_providers` index. Consequences now that the cursor reaches child rows:
- Single expanded provider: `get(child_idx)` → None → Enter is a silent no-op on those rows
  (parity gap vs TS, which starts login / edits profile / opens disconnect — listModeHandler.ts:120-153).
- Multiple expanded providers: `get(child_idx)` resolves to a DIFFERENT provider → Enter on one
  provider's child opens ANOTHER provider's Detail. This is precisely the PROV-102 class
  ("Enter on OpenAI profile shows Anthropic"), now made triggerable by PROV-103's nav fix.

**Resolution:** PROV-102 must dispatch Enter/`d` for ALL child NavItemKinds via
`focused_nav_item().kind` + `provider_id` (TS approach), eliminating the
`visible_providers()[selected_index]` fallthrough entirely (consistent with PROV-101 no-fallback
mandate). Scope of PROV-102 expanded from Profile-only to all child rows.

PROV-103 itself is correct and complete within its tested scope; this WARN is the domain of PROV-102.
