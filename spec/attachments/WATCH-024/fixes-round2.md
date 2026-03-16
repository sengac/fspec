# WATCH-024 Round 2 Post-Review Fixes

## Review Date: 2026-03-16
## Status: ✅ ALL FIXED

---

## 🔴 CATEGORY A: ChainOfCommand method parameter names — ✅ FIXED

- [x] **A1**: `add_supervisor(parent_id, watcher_id)` → `add_supervisor(subordinate_id, supervisor_id)`
- [x] **A2**: `remove_supervisor(watcher_id)` → `remove_supervisor(supervisor_id)`
- [x] **A3**: `get_supervisors(parent_id)` → `get_supervisors(subordinate_id)`
- [x] **A4**: `get_subordinate(watcher_id)` → `get_subordinate(supervisor_id)`
- [x] **A5**: `cleanup_parent(parent_id)` → `cleanup_subordinate(subordinate_id)`
- [x] **A6**: Internal shorthand vars `w2p`/`p2w` → `sup2sub`/`sub2sup`
- [x] **A7**: Local var `watchers` inside ChainOfCommand methods → `supervisors`
- [x] **A8**: Error message `"watcher already has a parent"` → `"supervisor already has a subordinate"`
- [x] **A9**: Comment `"Register a watcher for a parent session"` → `"Register a supervisor for a subordinate session"`
- [x] **A10**: Comment `"Get all watchers for a parent session"` → `"Get all supervisors for a subordinate session"`
- [x] **A11**: Comment `"Clean up all watcher relationships when a parent session is removed"` → `"Clean up all supervisor relationships when a subordinate session is removed"`

---

## 🔴 CATEGORY B: `create_watcher_session_with_id` not renamed — ✅ FIXED

- [x] **B1**: `create_watcher_session_with_id` → `create_supervisor_session_with_id`
- [x] **B2**: Call site in `session_create_supervisor` updated
- [x] **B3**: Parameter `parent_id: Uuid` → `subordinate_id: Uuid`
- [x] **B4**: Local vars `parent`/`parent_broadcast_rx` → `subordinate`/`subordinate_broadcast_rx`
- [x] **B5**: Comment updated
- [x] **B6**: Error `"Parent session not found"` → `"Subordinate session not found"`
- [x] **B7**: Comment `"Spawn watcher agent loop"` → `"Spawn supervisor agent loop"`
- [x] **B8**: Error log `"Failed to resolve credentials for watcher provider"` → `"...supervisor provider"`

---

## 🔴 CATEGORY C: `supervisor_inject` uses old variable names — ✅ FIXED

- [x] **C1**: Parameter `watcher_id: String` → `supervisor_id: String`
- [x] **C2**: Local `watcher_uuid` → `supervisor_uuid`
- [x] **C3**: Local `watcher` → `supervisor_session`
- [x] **C4**: Local `parent_uuid` → `subordinate_uuid`
- [x] **C5**: Local `parent` → `subordinate`
- [x] **C6**: Error `"Invalid watcher ID"` → `"Invalid supervisor ID"`
- [x] **C7**: Error `"Supervisor has no parent session"` → `"Supervisor has no subordinate session"`

---

## 🔴 CATEGORY D: `session_create_supervisor` NAPI function — ✅ FIXED

- [x] **D1**: Parameter `parent_id: String` → `subordinate_id: String`
- [x] **D2**: Local `parent_uuid` → `subordinate_uuid`
- [x] **D3**: Local `_parent` → `_subordinate`

---

## 🔴 CATEGORY E: SessionManager delegation methods — ✅ FIXED

- [x] **E1**: `SessionManager::add_supervisor(parent_id, watcher_id)` → `(subordinate_id, supervisor_id)`
- [x] **E2**: `SessionManager::get_supervisors(parent_id)` → `(subordinate_id)`
- [x] **E3**: Navigation comments updated
- [x] **E4**: `get_first_session()` comment updated

---

## 🔴 CATEGORY F: `supervisor_agent_loop` and related functions — ✅ FIXED

- [x] **F1**: `parent_broadcast_rx` → `subordinate_broadcast_rx` (all 3 functions)
- [x] **F2**: Doc comments updated
- [x] **F3**: Internal variables renamed (`watcher_session` → `supervisor_session`, etc.)

---

## 🔴 CATEGORY G: `navigation.rs` — ✅ FIXED

- [x] **G1**: Local `parent` → `subordinate`
- [x] **G2**: Comment `"Add the parent session"` → `"Add the subordinate session"`
- [x] **G3**: All comments updated

---

## 🔴 CATEGORY H: `useLazyConversationLines.ts` — ✅ FIXED

- [x] **H1**: Parameter `isWatcherView` → `isSupervisorView`
- [x] **H2**: Local `prevWatcherViewRef` → `prevSupervisorViewRef`
- [x] **H3**: Local `prevWatcherView` → `prevSupervisorView`
- [x] **H4**: Local `watcherViewChanged` → `supervisorViewChanged`
- [x] **H5**: JSDoc updated

---

## 🔴 CATEGORY I: Doc comments in production Rust code — ✅ FIXED

- [x] **I1-I14**: All production doc comments updated (parent→subordinate, watcher→supervisor)

---

## 🟡 CATEGORY J: Unstaged `index.d.ts` — ✅ FIXED

- [x] **J1**: Discarded formatting noise via `git stash push` + `git stash drop`

---

## 🟡 CATEGORY K: Test placeholder — ✅ FIXED

- [x] **K1**: Replaced `expect(true).toBe(true)` with meaningful assertions checking `create_watcher_session_with_id`, `cleanup_parent`, `watcher_inject` are all gone

---

## 🟡 CATEGORY L: `updateWorkUnitsFromWatcher` in fspecStore.ts — ✅ VERIFIED

- [x] **L1**: Confirmed this refers to the filesystem watcher (`work_units_watcher.rs`), NOT session watchers. Left as-is per spec rule: "Do NOT touch work_units_watcher.rs"

---

## Summary

| Category | Count | Status |
|----------|-------|--------|
| A: ChainOfCommand params | 11 | ✅ |
| B: create_watcher_session | 8 | ✅ |
| C: supervisor_inject vars | 7 | ✅ |
| D: session_create_supervisor NAPI | 3 | ✅ |
| E: SessionManager methods | 4 | ✅ |
| F: supervisor_agent_loop vars | 3 | ✅ |
| G: navigation.rs vars | 3 | ✅ |
| H: useLazyConversationLines | 5 | ✅ |
| I: Doc comments in prod Rust | 14 | ✅ |
| J: Unstaged index.d.ts | 1 | ✅ |
| K: Test placeholder | 1 | ✅ |
| L: fspecStore naming | 1 | ✅ |
| **TOTAL** | **61** | **✅ ALL FIXED** |
