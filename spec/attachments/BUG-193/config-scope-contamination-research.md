# BUG-193 Research — Config-Scope Contamination: Project Keys Mirrored into `~/.fspec/fspec-config.json`

**Status:** Research complete (discovery phase, BUG-193)
**Date:** 2026-09-26
**Reporter finding:** User's global `~/.fspec/fspec-config.json` contained project-scoped keys
(`"agent": "claude"` and a full `tools` section with Maven commands) — keys that, per the codebase's
own design, only belong in a project's `spec/fspec-config.json`.

---

## 1. The two-scope config model (CONFIG-008)

fspec uses **one shared JSON file name per scope** with a deep-merge load semantic, mirrored 1:1
from the deleted TypeScript reference (`src/utils/config.ts`):

| Scope | Path | Role |
|---|---|---|
| **User** | `~/.fspec/fspec-config.json` (`<data_dir>/fspec-config.json`) | Machine/user-level state: provider profiles, TUI preferences, RLCD settings |
| **Project** | `<project>/spec/fspec-config.json` | Per-repo state: test/quality tool commands, which agent harness was `init`ed |

Core module: `rust/common/src/fspec_config.rs`

- `load_config_with_dirs(data_dir, cwd)` → `deep_merge(user, project)` — **project overrides user**,
  recursively on objects, whole-value replace on arrays/scalars.
- `write_config_with_dirs(ConfigScope, value, data_dir, cwd)` → writes a **whole `Value`** to the
  chosen scope (pretty JSON, `preserve_order` serde feature keeps key order).

The load-side merge is correct and intended (project override wins, user provides defaults).
The bug is on the **write side**.

---

## 2. Root cause: read-MERGED, write-USER, wholesale

Two TUI persistence modules follow this exact pattern:

```
root = load_config_with_dirs(data_dir, cwd)   // ← DEEP-MERGED view (user + project keys)
root["tui"]["..."] = new_value                // mutate the target key
write_config_with_dirs(ConfigScope::User, &root, ...)  // ← writes the ENTIRE merged doc to USER scope
```

Because the read returns the **merged** view, the write-back persists **every project-scope key**
that happened to be in `<cwd>/spec/fspec-config.json` into `~/.fspec/fspec-config.json`.

### Affected write paths

| # | Module | Function | Key written | Contamination risk |
|---|---|---|---|---|
| 1 | `rust/sessions/src/mux_config_persistence.rs` | `save_mux_config_with_dirs` | `tui.mux` | **YES** — line 54: `load_config_with_dirs(data_dir, cwd)` → line 75: `write_config_with_dirs(ConfigScope::User, &root, ...)` |
| 2 | `rust/sessions/src/default_thinking_level_persistence.rs` | `save_default_thinking_level_with_dirs` | `tui.defaultThinkingLevel` | **YES** — line 60: `load_config_with_dirs` → line 85: `write_config_with_dirs(ConfigScope::User, ...)` |
| 3 | `rust/sessions/src/last_used_model_persistence.rs` | `save_persisted_model_string_to` | `tui.lastUsedModel` | No — reads the raw **user** file only (`read_config_value(user_dir/fspec-config.json)`) |
| 4 | `rust/tools/src/rlcd/config.rs` | `save_rlcd_url_at` | `rlcd.url` | No — raw user file only |
| 5 | `rust/sessions/src/profile_persistence.rs` / `profile_sections.rs` | profile/custom-model saves | `providers.*` | No — raw user file only |
| 6 | `rust/fspec-core/src/commands/configure_tools.rs` | `configure-tools` | `tools.*` | No — project scope only (never touches user file) |

**Triggers in the real binary:** any mux save (`/mux save`, dialog `s` commit, mux-exit
auto-save) and any `/thinking` default save, while `cwd` is a project whose
`spec/fspec-config.json` contains project keys. This happens on *every* mux-exit auto-save —
so one visit to a contaminated project permanently poisons the user config.

### Evidence

The reporter's `~/.fspec/fspec-config.json` contained:

```json
{
  "providers": { ... },          // legit user-scope
  "tui": { "lastUsedModel": "...", "mux": { ... } },  // legit user-scope
  "agent": "claude",             // ← project-scoped key (written by fspec init per project)
  "tools": {                     // ← project-scoped key (fspec configure-tools per project)
    "test": { "command": "JAVA_HOME=./vendor/jdk-17 mvn test" },
    "qualityCheck": { "commands": ["JAVA_HOME=./vendor/jdk-17 mvn -q compile"] }
  },
  "rlcd": { "url": "..." }       // legit user-scope
}
```

While the fspec repo's own project config (`/home/rquast/projects/fspec/spec/fspec-config.json`)
holds the identical `agent` + `tools` shape. The Maven values prove the user opened **another**
(Java/Maven) project in the TUI: its project-scope `tools` got mirrored into the user config on
the next mux/thinking save.

Note: the mirrored `tools`/`agent` in the user config are **dead data for every reader** —
no Rust reader consumes `tools` or `agent` from user scope (see inventory in CONFIG-009's
research doc), so the harm is pollution/confusion rather than misbehavior.

---

## 3. Why the bug survived

- The pattern was copy-propagated: `default_thinking_level_persistence` (TUI-092) explicitly
  "mirrors" the mux persistence design, and its doc comment states "load relies on
  `load_config_with_dirs`" — the merged view was **intentional for loading** but never
  scrutinized for the write-back half.
- The BUG-167 fix (`mux-config-persistence-wiring.feature`) pinned the read side ("load reads the
  deep-merged view" — R2) and sibling-key preservation, but its preservation examples
  (`bug167_mux_config_persistence_wiring.rs::seed_sibling_config`) seed `{"agent": "claude",
  "tools": ...}` **in the user file itself**, so the tests cannot detect cross-scope mirroring.
  No scenario asserts "project-scope keys do NOT appear in the user file after a save".
- `write_config_with_dirs` is a generic whole-value writer (CONFIG-008 interop primitive); the
  scope-correctness obligation lives entirely in the callers.

---

## 4. Proposed fix

**Principle:** *loads may read the merged view; user-scope writes must operate on the raw user
file only.* A write must never carry another scope's keys into a file.

### Fix A (preferred — minimal, local)

In the two affected save functions, replace the merged read with a raw user-scope read:

```rust
// BEFORE (mux_config_persistence.rs:54)
let mut root = load_config_with_dirs(data_dir, cwd).unwrap_or_else(|_| Value::Object(Map::new()));
// AFTER
let mut root = load_user_config_file(data_dir).unwrap_or_else(|_| Value::Object(Map::new()));
```

i.e. introduce a small `load_user_config_file(data_dir) -> Result<Value, String>` in
`codelet-common::fspec_config` (missing/empty → `{}`; invalid JSON → `Err`, same semantics as
`load_config_file`), and use it in the write half of
`save_mux_config_with_dirs` / `save_default_thinking_level_with_dirs`.

The `cwd` parameter stays (still needed for nothing after the change in these two functions —
it can be dropped from the signature, or kept for the log lines; dropping is cleaner and forces
callers to think). Load functions keep the merged view unchanged.

**Why not Fix B (strip project keys before write):** a blocklist of "project keys" duplicates the
ownership table, breaks the moment a new key lands in the wrong scope, and silently drops
legitimate user keys if the table is wrong. Fix A removes the contamination class entirely:
user-scope writes can only ever see/rewrite user-scope data.

### Migration / cleanup (one-off, manual)

Existing contaminated user configs: remove `agent` + `tools` (done for the reporter on
2026-09-26; backup at `~/.fspec/fspec-config.json.bak-20260926`). A future `fspec check`
warning (see CONFIG-009) would surface this for other users.

---

## 5. Affected code inventory (for the implementing developer)

**Write paths to change (2):**
- `rust/sessions/src/mux_config_persistence.rs` — `save_mux_config_with_dirs` (lines ~52–94)
- `rust/sessions/src/default_thinking_level_persistence.rs` — `save_default_thinking_level_with_dirs` (lines ~52–111)

**New helper (1):**
- `rust/common/src/fspec_config.rs` — `load_user_config_file(data_dir) -> Result<Value, String>`

**Call sites of the changed cores (compile-time driven, no behavior change):**
- `rust/fspec-tui/src/store/mux_state.rs` (lines 70, 95, 148)
- `rust/sessions/tests/tui092_shared_config_thinking_level.rs`
- `rust/fspec-tui/tests/bug167_mux_config_persistence_wiring.rs`

**Feature files whose acceptance criteria must be re-checked (not broken):**
- `spec/features/mux-config-persistence-wiring.feature` (BUG-167) — R2 pins "load reads the
  deep-merged view" (stays true); R3/R5 pin the save contract (still true: "preserving sibling
  keys" now means *user-scope* siblings only — the BUG-167 example that seeds `agent`+`tools`
  **in the user file** must be updated: after the fix those seeded user keys are still preserved,
  but the scenario's intent shifts to "user-scope siblings preserved").
- `spec/features/default-thinking-level-shared-config-persistence.feature` (TUI-092) — scenario
  "Project scope overrides the user value on load" stays valid; no save-side project-scenario
  exists to break.
- `spec/features/shared-config-merge.feature` (CONFIG-008) — core module contract unchanged
  (adds a new helper, does not alter `load_config_with_dirs`/`deep_merge`).

**Tests that will likely need new scenarios (ACDD red phase):**
1. Given a project `spec/fspec-config.json` with `tools` + `agent` keys, when a mux save
   happens, then the user file gains `tui.mux` and does NOT gain `tools`/`agent`.
2. Same for default-thinking-level save.
3. User-scope siblings (e.g. `providers`, `rlcd`, existing `tui.lastUsedModel`) survive a save.
4. Invalid-JSON user file: save degrades to `{}` (current behavior) — pin the new helper's
   error semantics.

---

## 6. Open questions

1. Should `save_rlcd_url_at` / profile saves be audited for the same class? — Done above (§2):
   they read the raw user file; no change needed. Re-audit if they ever switch to
   `load_config_with_dirs`.
2. Should `write_config_with_dirs(ConfigScope::User, ...)` be made unsafe-by-construction
   (e.g. a `UserScopeGuard` type that only raw-user reads can produce)? — Possible hardening
   follow-up; out of scope for BUG-193 (would ripple through all writers incl. TS-interop paths).
