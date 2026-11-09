# Worktree sessions and fspec spec/ state: research for the product owner

Question from the product owner: *"how to handle worktree work in fspec since the work unit and epic json etc isn't part of the worktree and merging them would also be painful.. look into the worktree code"*

Method: read-only reading of `~/projects/fspec` at HEAD `a27e92867`. Nothing was built, run or tested. Paths below are relative to `~/projects/fspec/rust/` unless they start with `spec/` or `.gitignore`. Some claims depend on runtime behaviour (for example, which `project_root` an LLM passes, or the daemon's process cwd). Those are marked **(runtime: unconfirmed)**.

---

## 0. TL;DR

- **Today:** an isolated session gets a full git checkout of `HEAD` at `<repo>/.fspec/worktrees/<session-id>/`, and that checkout includes its own copy of `spec/` (work-units.json, epics.json and so on). Where an fspec write lands depends on how the agent calls fspec:
  - **`fspec` CLI through the Bash tool:** the cwd is forced to the worktree, so the write goes to the **worktree's own copy**. The TUI never sees it, and the session's own stage gate keeps working from the stale main board.
  - **In-process `Fspec` tool:** `project_root` defaults to `"."`, which resolves against the **daemon's process cwd**, normally the main checkout. Board, features and coverage all go to **main**, outside the isolation boundary.
- **Merge back** is a plain file-level three-way merge (whole worktree vs. main working dir vs. base commit). It has no knowledge of spec state. A board that changed on both sides is a near-certain line conflict in a 14 MB file, because `meta.lastUpdated` and `prefixCounters` change on every write. The merge also writes main's spec files **without the spec lock and without an atomic replace**. And the merge commit tree **reverts main's commits to files the session never touched**, which hits work-units.json hardest because 476 of the last 550 commits touched it.
- **Recommendation: option (a), a shared state root.** Every worktree resolves board-class state (cards, IDs, epics, prefixes, tags, foundation, attachments, project config) to the main checkout's `spec/` through one resolver and one lock. Features, coverage and code-execution config travel with the branch. The pointer is a plain file in `<worktree>/.fspec/`, which the merge machinery already excludes, so the design does not depend on git or on any language.

---

## 1. How worktrees work today

### 1.1 Creation

- `SessionManager::create_isolated_session_with_id` calls `codelet_git::create_worktree(project, id)`, where `project` is the session's project (the main checkout): `sessions/src/session_manager/manager_impl_5.rs:26`. It then writes a manifest at `~/.fspec/git-sessions/<id>.json` recording `project_root`, `worktree_path` and `base_commit`: `manager_impl_5.rs:32-38`, `git/src/session_status.rs:47-84`.
- `create_worktree_at_ref` builds the worktree at `repo_path/.fspec/worktrees/<session_id>` (`git/src/worktree.rs:29,100`). It writes linked-worktree metadata with a detached HEAD (`worktree.rs:113,287-307`), **checks out every tracked file of the commit** (`worktree.rs:114`, `git/src/checkout.rs:34`) and initialises the index from that tree (`worktree.rs:115,251-285`).
- The base is **the `HEAD` commit, not main's working directory**. Uncommitted board edits in main are not in the worktree. In this repo the board is committed very often (476 of the 550 commits since 2026-11-01 touch `spec/work-units.json`), so the worktree's board starts stale and keeps getting staler.
- `.fspec/` is gitignored in this repo (`.gitignore:93`), so main never sees the worktree directory as untracked content. Independently, the untracked-file walk always skips any `.fspec` directory (`git/src/tree_utils.rs:186-189`). `spec/fspec-config.json` is gitignored (`.gitignore:41`), so **the worktree has no project config**. `spec/.fspec-write.lock` is gitignored (`.gitignore:111`). `fspec init` does not add `.fspec/` or the lock to a user project's `.gitignore` (nothing found in `fspec-core/src/commands/init*`).

### 1.2 Isolation boundary and the agent's cwd

- The binary registers the isolation callback at `fspec/src/common.rs:150`. `isolation_context` returns `worktree_path` (allowed) and `blocked_project_path = session.project` (the main checkout, blocked): `sessions/src/session_tool_callbacks.rs:106-123`.
- **Read, Write, Edit, Grep, Glob and Ls** validate and resolve paths against the worktree. Relative paths are joined to the worktree, and main-checkout paths are refused: `tools/src/facade/wrapper.rs:823-833, 877-1003`.
- **Bash:** session isolation overrides any `cwd` argument. Commands run in the worktree: `tools/src/bash.rs:73-79`.
- **The Fspec tool does no path validation.** `FspecToolFacadeWrapper::call` passes the LLM's `project_root` straight through to the handler: `tools/src/facade/wrapper.rs:1179-1256`. `project_root` defaults to `"."` (`tools/src/fspec.rs:34-43`). In the standalone binary the handler runs `codelet_fspec_core::dispatch_command` in-process (`agent-loop/src/fspec_handler.rs:43-59`, with `agent-loop/src/lib.rs:106` hard-wired `false`). Nothing rebases `"."` onto the session's effective cwd: no `project_root` rewriting exists in `agent-loop/src`. A relative `project_root` therefore resolves against the **daemon process cwd**.
- The environment reminder tells the LLM `Working directory: <daemon process cwd>` (`cli/src/session/context_gathering.rs:69-71,189`) plus `Worktree: .fspec/worktrees/<id>/` as a path relative to the project (`manager_impl_5.rs:69-79`). The LLM's choice of `project_root` is therefore ambiguous **(runtime: unconfirmed which value LLMs actually pass)**.

### 1.3 How fspec finds the project root (BUG-497 / BUG-523)

- `resolve_project_root(cwd)` lives in `common/src/project_root.rs:38-49`. Order: `cwd` itself if `cwd/spec` is a directory; else the parent of the nearest `spec/` walking up (max 10 levels) that sits next to a boundary marker (`.git`, `package.json`, `.gitignore`, `Cargo.toml`, `pyproject.toml`; `:12-18,52-72`); else the nearest marker directory.
- The CLI bridges use `spec_project_root()`, which calls `resolve_project_root(current_dir)`: `fspec/src/common/spec_project_root.rs:11-16`. `dispatch_command` re-resolves too: `fspec-core/src/dispatch.rs:241-246`.
- `lock_spec_dir` refuses a root whose spec dir is not `<root>/spec` (the BUG-523 guard): `fspec-core/src/io/spec_lock.rs:26-32,81-100`. Every loader builds `<root>/spec/<file>`: `fspec-core/src/io/ensure.rs:40-142`. Attachments go to `<root>/spec/attachments`: `fspec-core/src/commands/add_attachment.rs:233`.
- **From a worktree:** `cwd/spec` exists because spec/ is tracked, so the root is the worktree. From `worktree/rust/`, the walk-up finds `worktree/spec` first. **Edge case:** in a project whose `spec/` is *not* committed, the worktree has no `spec/`. The walk then goes `worktree → .fspec/worktrees → .fspec → <main repo>` and finds **main's** `spec/`, so that project already gets accidental shared state, features and coverage included.

### 1.4 The spec lock

- `SpecLock::acquire(dir)` takes an exclusive `flock` on `<canonical dir>/.fspec-write.lock`, reentrant per thread, with a 60 s timeout: `common/src/spec_lock.rs:43,100-129`. The lock is keyed by **the canonical spec directory** (`:104,112`).
- So `<main>/spec` and `<main>/.fspec/worktrees/<id>/spec` are **different locks**. Writers in the worktree and writers in main never contend, which is "correct" today only because they write different files. Coverage sidecars take the same lock as their spec dir (`fspec-core/src/io/spec_lock.rs:34-50`). The TUI's board writer (`move_work_unit`) takes the lock of `resolve_project_root(workspace)/spec`: `core/src/work_units_write.rs:47-52`.

### 1.5 Which spec/ the daemon and board read

- The daemon builds one `WorkUnitsWatcher::new(workspace)` for its start workspace (`fspec/src/common.rs:94-97`). Each opened workspace gets its own watcher (`rpc/src/workspaces.rs:188`). A worktree is never a workspace.
- The watcher reads `resolve_project_root(workspace)/spec/work-units.json` and watches that `spec/` directory **non-recursively**, reacting only to `work-units.json`: `core/src/work_units.rs:155-158,265,287-294,330`. **Writes to a worktree's board are invisible to the TUI.**
- Session-level card logic always reads **`session.project`, the main checkout**:
  - focus resolution: `sessions/src/activity_focus.rs:24-38`;
  - tool-admission receipts: `project = session.project` at `sessions/src/focus_reconciliation.rs:35`;
  - card validation for attribution: `tools/src/tool_admission.rs:101-125`, reading `project/spec/work-units.json` at `:107`;
  - focus reconciliation on board changes: `focus_reconciliation.rs:74-108`.
- The stage gate (BLOCK-006) takes the **stage** from that focus (main board) but matches paths, and loads `.fspec/stage-permissions.json`, relative to the **worktree**: `sessions/src/session_tool_callbacks.rs:135-148`, `tools/src/stage_gate.rs:59-67`, `tools/src/stage_permissions/mod.rs:69-71`. The project stage-permissions file lives under `.fspec/`, which is untracked, so it is absent in the worktree.
- After a successful Fspec tool call, focus is reconciled only if `canonical(request.project_root) == canonical(session.project)`: `agent-loop/src/fspec_handler.rs:60-81`. A worktree `project_root` skips reconciliation.

### 1.6 Merging back

- **TUI:** `/merge-worktree` runs inspect, then the confirm dialog, then `merge_session_worktree(FastForward)` (or discard): `fspec-tui/src/app/dispatch_merge_worktree.rs:45-161`. A conflict seeds the LLM with conflict context (`Action::SeedPendingInput`). Success tears the session down (WT-009).
- **Server side:** the repo is resolved from the session's project (WT-003), then `codelet_git::merge_session_with_strategy`: `sessions/src/handle_impl/handle_impl_8.rs:126-178`. Discard calls `discard_session`: `:179-185`, `git/src/session_status.rs:614-646`.
- **Algorithm** (`git/src/session_status.rs:514-559`, `git/src/session_result.rs:116-249`):
  1. Diff the base commit tree against the worktree's working files (`session_result.rs:56-96`).
  2. Snapshot **main's working directory**, not HEAD (`:145-149`).
  3. A file changed on both sides is a conflict (`git/src/session_diff.rs:195-244`). It gets a line-based diffy three-way merge (`git/src/three_way_merge.rs:44-58`), and unresolved hunks get conflict markers written **into the worktree file** for the LLM to fix (`session_result.rs:195-219`).
  4. Otherwise changed files are copied into main (`session_diff.rs:125-186`) and the worktree is deleted (`session_result.rs:246`).
  5. A commit is made on main (`git/src/merge_commit.rs`).
- Nothing in `git/src` treats `spec/` specially (grep for `"spec` returns nothing relevant). The spec JSON files are merged like any other text file.

---

## 2. Where each kind of spec write goes today

"CLI" means the agent runs `fspec …` through the Bash tool. The cwd is the worktree, so the root is the worktree (sections 1.2 and 1.3). "Fspec tool" means the in-process tool with `project_root` defaulting to `"."`, which is the daemon cwd and normally the main checkout **(runtime: unconfirmed)**.

| State | CLI in worktree | Fspec tool (`"."`) | TUI board shows it live? | Merge behaviour |
|---|---|---|---|---|
| Board `work-units.json` (cards, status, rules, questions, `prefixCounters`) | worktree copy (`ensure.rs:40-48`) | main | CLI: **no** (watcher watches main only, `work_units.rs:330`). Tool: yes | CLI: a 3-way line merge of a 14 MB file. It conflicts whenever main's board also changed, which is nearly always: every write bumps `meta.lastUpdated` (e.g. `set_user_story.rs:125`, `add_architecture_note.rs:146`) and `prefixCounters` (`create_bug.rs:235-238`) |
| `epics.json` | worktree | main | not watched at all | same, as plain text |
| `prefixes.json` | worktree | main | not watched | same |
| `tags.json`, `TAGS.md` | worktree | main | not watched | same; TAGS.md is derived and conflicts as text |
| `foundation.json`, `FOUNDATION.md` | worktree | main | not watched | same |
| Feature files `spec/features/*.feature` | worktree (correct: they belong with the code) | **main** (escapes isolation; Write/Edit to the same path would be refused) | n/a | CLI: merged with the code (good). Tool: already in main before review |
| `*.feature.coverage` | worktree (correct) | **main** (links test files that exist only in the worktree, so link/audit checks can fail or point at main's code) | n/a | as features |
| Attachments `spec/attachments/<ID>/` | worktree (`add_attachment.rs:233`) | main | n/a | CLI: new files copied on merge. The card's `attachments` list is in the board and conflicts with it |
| Spec lock | `worktree/spec/.fspec-write.lock` | `main/spec/.fspec-write.lock` | n/a | gitignored here, so not merged. In a user project that doesn't ignore it, it would show up as an added file |
| Project config `spec/fspec-config.json` | **absent** (gitignored, never checked out) | main | n/a | n/a |
| Status-change auto-checkpoints (`git stash create` + `refs/fspec-checkpoints/…`) | taken of the worktree (`update_work_unit_status/checkpoints.rs:7-13,24-60`) | taken of **main** (wrong tree) | n/a | refs live in the common git dir |
| Command hooks (`spec/fspec-hooks.json`, run with `cwd = project_root`, `fspec-core/src/command_hooks/executor.rs:89`) | run against worktree code | run against **main** code | n/a | n/a |
| Status gates: feature, test and coverage discovery and mtime checks (`update_work_unit_status/temporal.rs:56-67`, `coverage_gate.rs:41-49`) | read the worktree (correct) | read main (wrong: misses the worktree's new feature and tests) | n/a | n/a |

Consequences:

- **Two diverging copies of `work-units.json` exist as soon as a CLI-using agent writes in a worktree.** The main copy keeps moving (the TUI, other agents, the `chore(spec)` commits). The worktree copy was frozen at `HEAD` when the worktree was created.
- **The session's own guards read the main copy.** A CLI agent that runs `update-work-unit-status X implementing` in its worktree changes only the worktree board. Its focus, read from main, stays at `testing`. The stage gate then keeps refusing implementation writes (`session_tool_callbacks.rs:135-148`, `stage_gate.rs:59-75`). Attribution to a card created in the worktree fails with "unknown card … in invoking project" (`tool_admission.rs:107-118`). **(runtime: unconfirmed, but this is the direct reading of the code.)**
- **The two call paths disagree.** The same agent can write the board to main through the tool and to the worktree through the CLI. Its features and coverage likewise land in main through the tool and in the worktree through the CLI.

---

## 3. The real problems

1. **Diverging boards.** Covered in section 2. Nothing reconciles them except the file merge.
2. **Merge conflicts in a 14 MB JSON file.** The line-based diffy merge (`three_way_merge.rs:44-58`) gives a guaranteed conflict on `meta.lastUpdated` (line 3) and on the `prefixCounters` and `states` arrays whenever both sides wrote. The LLM is then asked to remove conflict markers from a 14 MB file with Edit. A "clean" textual merge can still produce invalid or semantically wrong JSON: duplicate IDs, a card listed in two `states` arrays, or a lost `stateHistory` entry. No JSON validation happens on merge.
3. **Agents in two worktrees editing the same card.** With per-worktree boards, both edits merge at file level, last-merge-wins or conflict. Transitions are never re-validated against the other side's state.
4. **Features and coverage belong with the code**, but the Fspec tool path writes them to main immediately, before review and outside isolation (`wrapper.rs:1179-1256` has no path check). The CLI path gets this right.
5. **The lock does not work across worktrees.** It is keyed by canonical spec dir (`common/src/spec_lock.rs:104,112`). Worktree and main writers never contend, and the merge's write of `main/spec/work-units.json` takes **no lock at all** and uses a **non-atomic** `fs::write` (`session_diff.rs:146` → `git/src/tree_snapshot.rs:146-166`). That bypasses the BUG-477 protocol. A concurrent TUI move or agent write can be lost, and the watcher can read a half-written 14 MB file (BUG-539 only masks this as a transient empty board).
6. **Attachments.** The files and the card's `attachments` list sit in different files. A per-worktree board merges them separately, so dangling references are possible.
7. **ID counters.** `create-bug` and `create-story` allocate `max(prefixCounters[prefix], max existing suffix) + 1` under the lock of *their* spec dir (`create_bug.rs:24-25,235-238`). Two worktrees, or one worktree and main, both allocate `BUG-579`. After a merge you get duplicate IDs, or a JSON conflict that hides the collision. Renumbering would mean rewriting feature tags, coverage, attachment dirs and commit messages.
8. **The watcher misses worktree writes.** It watches main `spec/` non-recursively (`work_units.rs:330`). The board, focus, stage gate and admission all read main.
9. **Merge-commit tree bug (found while reading; not covered by any test).** `commit_merged_session` starts from main's `HEAD` tree and overlays **every** blob of the post-apply worktree snapshot (`git/src/merge_commit.rs:56-64`). The snapshot holds every tracked path, with untouched files at **base** content (`tree_utils.rs:49-107`). If main's HEAD moved after the worktree was created, every file main changed but the session didn't is **reverted in the merge commit**. Main's working tree and index keep the newer content (only delta paths are synced, `merge_commit.rs:95`), so `git status` shows a staged "revert of the revert". The comment at `:56-58` assumes HEAD == base. `git/tests/wt008_merge_commit.rs` never advances HEAD. In this repo `work-units.json` moves on main almost every commit, so a merged session would roll the committed board back in history.
10. **The worktree lacks project config.** `spec/fspec-config.json` and `.fspec/stage-permissions.json` are untracked, so isolated sessions run with different config from main.
11. **Discard semantics.** With per-worktree boards, discarding silently throws away card progress. With shared state, it leaves cards whose status claims work that no longer exists. Neither case is surfaced today.
12. **Fspec tool `"."` is anchored on the daemon cwd, not on the session.** This also affects non-isolated sessions in a non-start workspace (multi-workspace, BUG-427). Their Fspec tool calls hit the start workspace's board.

---

## 4. Options

Each option is assessed on shared vs. branch files, concurrency, IDs, discard, merge-flow changes, and language/VCS neutrality. All options must respect the earlier decisions: no legacy or fallback paths, the 13 MB board stays one file (splitting was rejected), and fspec stays language-agnostic.

### (a) One state root shared by every worktree; features and coverage travel with the branch

**Mechanism.**
- When fspec creates an isolated copy, it writes a pointer file inside the copy, e.g. `<worktree>/.fspec/state-root`. The file holds the absolute path of the main project root (and the session id).
- `.fspec/` is excluded from every session diff, conflict check, apply and commit by construction (`tree_utils.rs:186-189`), so the pointer never merges.
- The resolver in `codelet-common` grows from one root to two: `ProjectRoots { content, state }`.
  - `content` = today's `resolve_project_root(cwd)`: the worktree, created as an empty `spec/` there if the copy has none.
  - `state` = the pointer's target when a pointer exists, otherwise `content`.
- No fallback: a pointer that is unreadable, relative, chained (its target has its own pointer), or whose target lacks `spec/` is a hard error naming the file.

**Shared (state root) vs. travels (content root), and why:**

| Shared at the state root | Why |
|---|---|
| `work-units.json` (cards, `prefixCounters`, `states`), `epics.json`, `prefixes.json`, `tags.json` + `TAGS.md`, `foundation.json` + draft + `FOUNDATION.md`, `attachments/`, `schedules.json`, `schedule-log.jsonl`, `fspec-config.json`, `.fspec/stage-permissions.json` and other project `.fspec/*` config, the lock | These are registries. Their integrity is global: unique IDs, one status per card, one tag registry. Attachments are referenced by cards. Config should be the same everywhere. None of it describes a code change. |

| Travels with the branch (content root) | Why |
|---|---|
| `spec/features/*.feature`, `*.feature.coverage`, `spec/fspec-hooks.json`, docs and skills under `spec/` | These describe or verify the code. Coverage names test files and line ranges that exist only on the branch. Hooks run the branch's scripts. All of it must be reviewed and merged with the code, and discarded with it. |

`spec/fspec-hooks.json` is a judgment call for the product owner. It is tracked and drives code execution, so the recommendation is to keep it with the branch.

**Commands need both roots.** Status gates read the card from `state` and features, tests, coverage and mtimes from `content` (`temporal.rs:56-67`, `coverage_gate.rs:41-49`). Hooks and auto-checkpoints run in `content` (`executor.rs:89`, `checkpoints.rs:7-13`). `link-coverage` validates against `content` and writes the sidecar there. `audit` reads the board from `state` and everything else from `content`. This is the main cost: roughly 140 commands build `<root>/spec/...` paths (per BUG-523's notes). The fix is mechanical: one `SpecPaths` type with `work_units()`, `epics()`, `features_dir()`, `coverage_for()` and so on, plus a source-shape test banning `.join("spec")` outside that module. BUG-523 already proved this pattern.

**Concurrency and locking.** One lock: `state/spec/.fspec-write.lock`. Every spec write takes it, including content-root feature and coverage writes and the merge's apply step. A single lock avoids lock-ordering deadlocks. Writers in main, in any worktree and in the TUI all contend on the same inode (flock works across processes on one host). The board watcher already watches the state root, so the TUI updates live with no change. Focus, stage gate and admission already read `session.project`, which is the state root, so they become consistent instead of stale.

**IDs.** A single `prefixCounters` under one lock, so duplicates are impossible. A discarded worktree just burns the IDs it allocated, which is harmless.

**Discard.** Board changes made from the worktree persist. The code, features and coverage vanish, so cards may claim `testing` or `implementing` with no feature in main. The discard flow must name the cards whose status the session changed and offer to move each back to its pre-session status through a normal backward transition. That needs `stateHistory` entries to record the session id when written through a pointer. Rules, questions and examples added during specifying stay, because they are still valid knowledge.

**Merge flow changes.**
1. Shared-state paths are excluded from diff, conflict detection, apply and commit.
2. If the worktree's checked-out copy of a shared file differs from base, the merge **refuses** and names the files. A difference means something bypassed the resolver; there is no silent fallback.
3. Apply holds the state-root lock and uses `replace_file_atomically` for spec files (`common/src/spec_lock.rs:212`).
4. The merge commit overlays only the session's delta (problem 9).
5. The merge commit contains code, features and coverage only. Board commits stay with main's normal flow (the existing `chore(spec): card …` commits).

**Language-agnostic and outside git.** The pointer is a plain file. Any isolated copy can carry one: a git worktree, a jj workspace, `cp -r`, a container bind mount. Without a pointer, behaviour is exactly today's single root. Nothing in the resolver depends on git. Optionally, a small `fspec` command can write or show the pointer for copies made by hand.

**Residual issues.**
- The worktree still holds a stale checked-out copy of `work-units.json` and friends. Nothing reads it through fspec, but an agent's `Grep` would. The worktree's `AGENTS.md` and system reminder should say so.
- A card worked in a worktree has its feature only in the worktree. Moving that card from main (for example, a TUI drag to `validating`) fails the feature gate. That is the correct, loud outcome.

### (b) Per-worktree board with a structured merge

- **Shared vs. branch:** everything travels. Merge needs a JSON-aware three-way merge keyed by card id and field, with semantics for `states` arrays (one state per card), `stateHistory` (union, ordered), rules and examples (union with ids), `prefixCounters` (max), and similar logic for epics, tags and foundation.
- **Concurrency:** no cross-worktree locking is needed while working. The session's guards (focus, stage gate, admission) and the TUI still read main, so they must be re-pointed at the worktree board per session. The TUI then shows a different board per session, or a merged view.
- **IDs:** collisions are unavoidable without a central allocator. The merge must renumber and rewrite every reference: feature tags, coverage, attachment directories, commit messages, dependencies, epic membership. Otherwise IDs need a central allocator anyway, which is option (a)'s lock.
- **Discard:** clean. All board progress vanishes with the branch. That is arguably wrong for discovery work: Example Mapping done in a discarded session is lost.
- **Merge flow:** a custom merge driver for each spec JSON file. Transitions are not re-validated (two sessions can both move a card legally from their own view). Conflicts still need human or LLM resolution, just at field level.
- **Neutrality:** a git-independent merger is possible but large.
- **Verdict:** it is the most code and the most semantics to get right, and it still needs central IDs. It does not answer "TUI board updates live".

### (c) The daemon owns board state; worktree agents talk to it

- **Shared vs. branch:** same split as (a). Board writes become RPCs to the daemon, a single in-memory writer that persists to `spec/`.
- **Concurrency:** strong. One writer, no re-parsing 14 MB per command, push events.
- **IDs:** central.
- **Discard / merge:** same as (a).
- **Neutrality:** fine. **But** the `fspec` CLI is used without a daemon: Claude Code agents, CI, and plain shells (this research bot is one). Either the daemon becomes mandatory for every board write, or the CLI keeps a direct-file path, which is exactly the two-path fallback the product owner rejected.
- **Verdict:** a possible later optimisation **on top of (a)**. The resolver decides *where* the state is; a daemon could later decide *who* writes it. It is not a substitute.

### (d) Other options the code suggests (considered and rejected)

- **(d1) Symlink the shared files in the worktree to main's.** `replace_file_atomically` renames a temp file over the target (`common/src/spec_lock.rs:27-30,212`). The rename replaces the symlink with a regular file, so the first write silently forks the board again. The merge would also see a blob-to-symlink change and could replace main's board with a symlink. Windows symlinks need privileges. Rejected.
- **(d2) Keep per-worktree boards and exclude them from merge.** This loses all card progress made in the session. Rejected.
- **(d3) A command journal replayed at merge** (the worktree records fspec commands and the merge replays them against main under the lock). Transitions would be re-validated. But visibility is deferred (no live TUI), replayed `create-*` calls get new IDs (so references need rewriting again), and replay failures are hard to explain. Rejected in favour of (a), which gets the same validation immediately.
- **(d4) Git `skip-worktree` or sparse checkout to keep spec JSON out of the worktree.** Git-specific, so it breaks neutrality. Deleting the files instead would show as deletions in the diff, and the merge would delete main's board. Rejected.
- **(d5) Git common-dir discovery** (worktree → `.git` file → common dir → main workdir) as the resolver mechanism. It would work for fspec-made worktrees, but it is git-only and would not cover jj or plain copies. The pointer file is the neutral version. The session manifest already records `project_root` (`git/src/session_status.rs:58`), but a CLI can't find the manifest from its cwd without knowing the session id.

---

## 5. Recommendation

Adopt **(a)**, the shared state root, as a set of focused cards, and fix the two merge bugs (problems 5 and 9) regardless of which option is chosen, because they corrupt main today. Keep (c) on the shelf as a later performance step that sits behind the same resolver.

### Draft cards

All cards use prefix `WT` ("Worktree session-isolation & merge lifecycle bugs"; the counter is at 12) and epic `worktree-isolation`.

**1. WT-013: Isolated copies share the main checkout's board through one state-root resolver**

When fspec creates an isolated session it writes `<worktree>/.fspec/state-root`. Every fspec reader and writer (CLI bridges, `dispatch_command`, codelet-core board writer, watcher, focus, admission) then resolves `ProjectRoots { content, state }` through `codelet-common`. Board-class files are read and written at the state root; features, coverage and hooks at the content root. Status gates, audit and link-coverage read the card from `state` and features, tests and coverage from `content`. Hooks and auto-checkpoints run in `content`.

Rules:
- Shared at the state root: work-units, epics, prefixes, tags + TAGS.md, foundation (+ draft) + FOUNDATION.md, attachments, schedules and schedule log, fspec-config.json, project `.fspec/` config.
- Travel with the branch (content root): `spec/features/**`, `*.feature.coverage`, `spec/fspec-hooks.json`.
- A missing pointer means one root (state = content).
- A pointer that is unreadable, relative or chained, or whose target has no `spec/`, is a hard error naming the file. No fallback.
- One lock: `state/spec/.fspec-write.lock` guards every spec write in either root.
- All spec paths come from one `SpecPaths` module. A source-shape test fails on any `.join("spec")` outside it.
- No git dependency in the resolver. Any directory with a pointer behaves the same.

**2. WT-014: The Fspec tool resolves its project root from the session, not the daemon's working directory**

A relative `project_root` (including the default `"."`) is joined to the session's effective cwd: the worktree for isolated sessions, the session project otherwise. Resolution then goes through WT-013. Today `"."` resolves against the daemon process cwd (`tools/src/fspec.rs:42`, `agent-loop/src/fspec_handler.rs:47-51`). Isolated sessions therefore write features and coverage into main, and sessions in a non-start workspace hit the start workspace's board.

Rules:
- Relative roots are anchored on the session's effective cwd.
- An absolute root outside the session's content or state roots is refused, the same as the Write tool.
- Focus reconciliation compares the resolved **state** roots, not raw `project_root` strings (`fspec_handler.rs:60-66`).
- The environment reminder's `Working directory` is the session's effective cwd.

**3. WT-015: Merging a worktree never merges shared spec state**

The session diff, conflict detection, apply and merge commit exclude the shared-state paths defined in WT-013. If the worktree's checked-out copy of a shared file differs from its base content, the merge is refused and the files are named, because the change means something wrote around the resolver.

Rules:
- Shared paths never appear in inspect counts, the confirm dialog, conflicts or the merge commit.
- A modified shared file aborts the merge before anything is copied.
- Features, coverage and hooks merge like code.
- The exclusion list comes from the same definition the resolver uses (one source).

**4. WT-016 (bug): The merge commit reverts main's commits to files the session never touched**

`commit_merged_session` overlays every blob of the post-apply worktree snapshot onto main's HEAD tree (`git/src/merge_commit.rs:56-64`). Files main changed after the worktree's base commit are written back at base content in the merge commit, and `git status` shows staged changes afterwards.

Rules:
- The commit tree is HEAD's tree plus only the session's added, modified and deleted paths.
- Add a test where main commits a change to an untouched file (and to `spec/work-units.json`) after the worktree is created. After the merge, `git show HEAD:<file>` equals main's version and `git status` is clean.

**5. WT-017 (bug): Applying a merge writes spec files without the spec lock or an atomic replace**

`apply_snapshot_to_main` writes with `fs::write` (`git/src/tree_snapshot.rs:156`), with no `SpecLock`. A concurrent board write can be lost, and the watcher can read a partially written file.

Rules:
- Applying a merge holds the state-root spec lock for the whole apply.
- Every file under `spec/` is written with `replace_file_atomically`.
- A lock timeout fails the merge with the standard lock message and nothing applied.

**6. WT-018: Discarding a worktree names the cards its session changed**

Under a shared board, card progress made from a worktree survives a discard while its features and code do not.

Rules:
- `stateHistory` entries written through a state-root pointer record the session id.
- The discard confirmation lists every card whose status that session changed, with its status before the session's first change.
- The user can move each card back with a normal backward transition. Nothing is reverted silently.
- Cards the session created are listed and kept; their IDs are never reused.
- Rules, questions and examples are kept.

**7. WT-019 (optional): A card is worked in at most one isolated session at a time**

Two isolated sessions focusing the same card would write competing features and coverage that only collide at merge.

Rules:
- Setting focus on a card already focused by another live isolated session is refused, naming that session.
- Closing, merging or discarding the session releases the card.
- Non-isolated sessions are unaffected.

### Things I could not confirm (runtime)

- Which `project_root` LLMs actually pass to the Fspec tool in isolated sessions, and the daemon's process cwd in typical launches. The code says relative paths resolve against the process cwd.
- Whether the stage gate visibly blocks a CLI-using worktree agent after it advances status. The code path says it does (the stage comes from the main board).
- Whether any user project gitignores `.fspec/` and `spec/.fspec-write.lock`. `fspec init` doesn't add them.
- The merge-commit revert (problem 9) was found by reading the code; I did not reproduce it.
