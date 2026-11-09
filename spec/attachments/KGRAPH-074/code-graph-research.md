# fspec code graph (AST graph / GraphSearch): research report

Date: 2026-11-07. Scope: `rust/graph` (crate `codelet-graph`), its nanograph dependency, the GraphSearch tool wiring in `rust/agent-loop` and `rust/tools`, and the original KGRAPH cards. All work was read-only in `~/projects/fspec`. Clones, probes and benchmarks were built and run only under `/tmp`.

---

## 1. Cards found

The AST "code graph" lives in the **KGRAPH** prefix, epic `knowledge-graph`. GRAPH-001..003 and VIZ-* are separate, older backlog cards about the *work-unit* dependency graph, not the code graph.

| Card | Status | What it is |
|---|---|---|
| KGRAPH-001 | backlog | Nanograph knowledge graph: architecture and research (umbrella). |
| KGRAPH-013 | done | Dual-graph refactor: AST graph plus Learnings graph. |
| **KGRAPH-014** | done | **Parent: AST Connection Graph.** Promises "zero LLM cost", "rebuilt on demand", "project-scoped (one per working directory)" and "<10MB per 100K LOC". Attachments: `spec/attachments/KGRAPH-014/01-ast-connection-graph-research.md`, `ast-research-parent-card-review.md`. |
| KGRAPH-016 / 017 / 018 / 019 | done | Data model and schema; extraction pipeline (ast-grep); dependency population; query interface. |
| KGRAPH-025 | done | Multi-language extractors (14 languages) and 11 dependency-manifest extractors. |
| KGRAPH-026 / 055 / 070 / 071 | done | Bug fixes: duplicate File stubs violating `@unique`; dangling Function refs from Python/Java class imports; the Dart `extension` enum; no way to force a rebuild after a schema change (added `reset`). |
| KGRAPH-027 | done | Dead-code detection (Calls/TypeRef edges plus an orphan query). |
| KGRAPH-056 / 057 | done | Dart support. |
| **KGRAPH-059** | done | **Parent: "GraphSearch Enhancement — CodeGraphContext Feature Parity Analysis".** Names the reference project. Attachments: `cgc-feature-parity-analysis.md`, `ast-research-all-children-complete.md`, `review-findings.md`. |
| KGRAPH-060 | done | Call chain (path tracing). |
| KGRAPH-061 | done | Transitive callers and callees. |
| KGRAPH-062 | done | Cyclomatic complexity. |
| KGRAPH-063 | done | Source and metadata stored in nodes. |
| KGRAPH-064 | done | Class hierarchy and inheritance traversal. |
| KGRAPH-065 | done | "Live File Watching and Incremental Re-indexing". It is really mtime-based incremental only: no watcher exists. |
| KGRAPH-066 / 067 / 068 | done | Variables; full-text search; decorators. |
| KGRAPH-069 | done | Portable bundles (export/import). |
| RPC-092 | done | Verbatim lift of `rust/napi/src/graph/` (52 files, ~15K LOC) into the `codelet-graph` crate. |
| CLEAN-006 | backlog | "Dead code removal — AST graph-verified orphan files and unreachable functions". **Risky given bugs B08–B11 below.** |
| BUG-542 / BUG-574 | done / implementing | Test-harness issues in `codelet_graph_crate_lift.rs`. |

Feature files read:
- `incremental-reindexing.feature`
- `class-hierarchy-and-inheritance-traversal.feature`
- `transitive-callers-callees-multi-hop-traversal.feature`
- `call-chain-path-tracing-between-two-functions.feature`
- `ast-dead-code-detection.feature`
- `ast-graph-data-model.feature`
- `ast-extraction-pipeline.feature`
- `graph-database-reset.feature`
- `portable-graph-bundles.feature`
- `source-code-metadata-storage.feature`
- `variable-symbol-tracking.feature`
- `decorator-annotation-search.feature`
- `cyclomatic-complexity-analysis.feature`

---

## 2. The reference project

- **Project:** CodeGraphContext (CGC), a Python MCP server and CLI that puts code into Neo4j, KùzuDB or FalkorDB.
- **URL:** https://github.com/CodeGraphContext/CodeGraphContext. Named in `spec/attachments/KGRAPH-059/cgc-feature-parity-analysis.md`: "Version analysed: 0.3.1", originally cloned to `/tmp/CodeGraphContext`, which no longer exists.
- **Clone:** `/tmp/graph-reference/CodeGraphContext`
  - HEAD = `6ed71a9f03f8eefec5bf7c38d638425efdf92ffe` (2026-11-05; latest tag v0.5.7).
- **Version the cards used:** a git worktree of tag `v0.3.1` at `/tmp/graph-reference/CodeGraphContext-v0.3.1`, commit `3f781847360da37e5b93b6c0e847874ba35942cf`.
- **Storage engine** (cloned for the upgrade section): https://github.com/nanograph/nanograph at `/tmp/graph-reference/nanograph`.
  - HEAD `972a98703bd5`.
  - v1.0.0 = `67b8c68c`, v1.3.0 = `d0223f10`.
  - nanograph is the storage engine, not the reference design.

---

## 3. How fspec's code graph works today

**Entry point.** The GraphSearch tool (`rust/tools/src/graph_search/`) is registered for every provider agent (claude/openai/gemini/zai/codex/copilot). Its handler is `rust/agent-loop/src/graph_search_handler.rs`:
- `create_handler()` gets one closure per session.
- The closure calls `block_in_place` and then `block_on`.
- It dispatches on `GraphSearchAction`: `ast_search | ast_neighbors | ast_stats | ast_index | ast_dead_code | ast_call_chain | ast_callers | ast_callees | ast_hierarchy | ast_complexity | ast_export | ast_import` plus the `learnings_*` actions.
- There is no RPC, CLI or TUI surface for the code graph; it is agent-tool only. DeepSearch adds GraphSearch only when a graph is already open in the process.

**Where it is stored.** `registry.rs` keeps a process-global `HashMap<name, GraphDatabase>`.
- `"ast-code"` resolves to `std::env::current_dir()/.fspec/graph/ast-code.nano` (`registry.rs:680-699`).
- `"learnings"` resolves to `~/.fspec/graph/learnings.nano`.
- On first access it opens or inits with a schema-text SHA-256 check (`database.rs:284-314`). It never closes until process exit.

**Storage format.** nanograph 1.0.0, which is Lance datasets plus `graph.manifest.json`, `_wal.jsonl`, `schema.pg` and `schema.ir.json`.
- In 1.0.0, `Database::open` loads the **whole graph into memory** (`DatabaseShared.storage: RwLock<Arc<GraphStorage>>`, nanograph-1.0.0 `src/store/database.rs:261-273`).
- Schema (`rust/graph/schemas/ast-code.pg`):
  - Node types: File, Module (never populated), Function, Type, Variable, Dependency.
  - Edge types: Contains, ContainsType, ContainsVariable, Imports, Calls, Implements, Extends, TypeRef, DependsOn.
  - Function and Type nodes carry `source` (capped at 100 lines / 4 KB, `metadata.rs:71-74`), docstring, decorators and parameters.

**What gets parsed** (`ast_pipeline/mod.rs`):
- The walk uses `ignore::WalkBuilder`: hidden files skipped, `.gitignore` honoured (except when a custom `path` is given), plus a hard-coded `SKIP_DIRS` list.
- Files are filtered against 28 extensions in 14 languages.
- Each file goes to a per-language extractor that is built on **ast-grep pattern strings** (`find_all("fn $NAME($$$ARGS) { $$$BODY }")`, …), wrapped in `catch_unwind`.
- Per file, an extractor produces:
  - File / Function / Type / Variable nodes, keyed by `slugify_path(rel)` and `"{file_slug}::{name}"`;
  - Contains edges;
  - Imports edges, resolved per language against `known_files`;
  - Calls and TypeRef edges, found by **scanning function body text** for `ident(` (`edge_helpers.rs:78`) and resolving the names against same-file names plus a relative-import map.
- Separately, 11 manifest extractors produce Dependency nodes and DependsOn edges.
- `deduplicate_entities` (`mod.rs:288-415`) merges same-key nodes ("more properties wins") and **drops every edge whose endpoint does not exist**.

**Indexing modes** (`ast_index.rs`):
- **Full:**
  1. walk;
  2. extract every file sequentially;
  3. walk again to stat mtimes;
  4. `load_with_mode(Overwrite)` of the whole entity set.
- **Incremental:**
  1. walk and stat;
  2. `export_all_entities()`, which exports the whole DB (including all source text);
  3. read the stored `lastModified`;
  4. partition files into changed / new / deleted;
  5. if there is no prior index or more than 50% changed, silently do a full index;
  6. otherwise `export_all_entities()` **again**, keep the entities "owned" (by slug prefix) by unchanged files, re-extract the changed files, deduplicate, and Overwrite-load **the whole graph** again.
- **Triggering:** nothing calls `ast_index` automatically. There is no watcher, no scheduler hook and no startup indexing. Only an agent calling `GraphSearch{action:"ast_index"}` triggers it.

**Queries:**
- Every query goes through `Database::run_json(full .gq source, name, params)`. In 1.0.0 that **re-parses the whole 222-line `.gq` file**, type-checks, plans and executes on every call (`query_input.rs:256`, `database.rs:553-570`).
- `ast_search`: runs `all_functions` / `all_files` / `all_types` / `all_dependencies` / `all_variables`, which return **every row with `source`**, then filters substrings in Rust and stops at `limit`.
- `ast_neighbors`: 14 sequential single-hop queries, regardless of the node's type.
- `ast_call_chain` / `ast_callers` / `ast_callees`:
  1. `GraphSnapshot::load` (all files plus all functions with source);
  2. `build_adjacency_list`, which runs **one `function_calls` query per function in the graph**;
  3. BFS in Rust.
- `ast_hierarchy`: BFS over Extends/Implements edges, plus "methods = every function in the type's file".
- `ast_dead_code`: `not {}` anti-joins, then Rust-side filters for path, test status and Flutter.
- `ast_complexity`: `all_functions`, then a sort.

---

## 4. Port vs reference (CGC v0.3.1, with fixes up to HEAD)

| Aspect | CGC v0.3.1 | CGC HEAD (v0.5.7) | fspec port | Assessment |
|---|---|---|---|---|
| Parser | tree-sitter queries per language; walks the full AST | same, plus a pipeline rewrite | ast-grep *pattern strings*; body text scanned for calls | fspec dropped real AST traversal. It misses generic, const and unsafe Rust fns (B10), is roughly 1,700x slower than a kind-based DFS (P1), and its call detection is lexical (B08). |
| Indexing passes | 2 passes: a pre-scan builds a global `imports_map` (name → paths), then CALLS and INHERITS are linked **after all files are written** | same; resolution sorted and made linear (`09b6db79`, 257.6s → 6.5s) | 1 pass. Each file resolves its own calls with only a per-file import map plus `known_files` | fspec has no global link phase. Cross-file edges depend on per-file guesses, and incremental mode cannot re-link (B03). |
| Symbol identity | `(name, path, line_number)` composite unique; but MATCHes ignore the line, so edges cross-multiply | `(name, path, line_number, occurrence_index)`; both caller and callee pinned to a line | `file_slug::name`, with a lossy `slugify_path` | fspec drops same-named functions entirely (B04), and different paths can collide (B05). Worse than both CGC versions. |
| Call resolution | local → imports_map (unique) → `candidates[0]` → "relaxed global" fan-out to every same-named function | 9 confidence tiers; ambiguous calls skipped; tier ≥8 written as `HEURISTIC_CALLS`; `self.m` scoped to the class; language filter | bare `ident(` only. No method calls (`x.f()`, `self.f()`, `Type::f()`, `mod::f()`). Only relative imports. Uppercase first letter means TypeRef | fspec avoided CGC's fan-out (good: no false positives from that source) but misses most real edges in OO and Rust code (B08, B09). |
| Inheritance | INHERITS resolved after indexing; C# IMPLEMENTS guessed | fixed hierarchy queries | **never extracted**. Schema edges exist but nothing produces them | `ast_hierarchy` is dead on real code (B06). |
| Incremental / watcher | watchdog with a 2s debounce; on every save, delete the file subtree, re-add it, re-parse **every** file and re-link everything (O(repo) but correct) | per-file under an RLock; re-links callers found via existing inbound edges; still misses "a new definition now resolves an old call" | mtime diff with no watcher; reuses old edges owned by unchanged files; whole-graph Overwrite | fspec's incremental mode is neither cheap (export plus overwrite of everything) nor correct (B03, B14). |
| Storage writes | one `session.run` per node and edge (slow) | UNWIND batches (claimed ~1400x) | a single JSONL batch with Overwrite. Cheap per write, but always the whole graph | fspec is fine for full index; wasteful for incremental (P4). |
| Multi-hop queries | Cypher `[:CALLS*]`, unbounded (explodes) | `*1..depth`, clamped to 20 | N single-hop queries, one per function, then a Rust BFS | fspec is O(N) queries per request (P2). nanograph 1.0.0 *does* support bounded `calls{1,N}` (§9), contrary to the KGRAPH-059 note. |
| Dead code | zero CALLS, minus name/decorator exclusions | File-sourced callers counted; substring filter fixed | zero Calls; `isTest` from a path substring; count capped | With fspec's incomplete edges, dead code is mostly false positives (B11). |
| Complexity | tree-sitter node counting (Python and 9 others); other languages default to 1 | same | keyword counting over text for all 14 languages | fspec covers more languages, but heuristically (B23). |
| Ignore rules | `.cgcignore` only, no `.gitignore` | `.gitignore` plus `.cgcignore`, with negations; symlink-safe; sorted walk | `.gitignore` plus SKIP_DIRS; disabled for custom paths | Roughly equal. fspec's walk is not sorted, but the Overwrite load makes order mostly irrelevant. |
| Graph scope | per repo; multiple repos in one DB, keyed by `Repository.path` | same | **one DB per process cwd**, regardless of the session's project | B01 is fspec-only. |
| Bundles | zip of jsonl; edges keyed by element_id | fixes | zip with schema and JSONL keyed by slug | OK. |

Things CGC had that fspec dropped and that matter:
1. The global link phase after all symbols are known.
2. Line-pinned identity.
3. Class-scoped method containment (`Class -CONTAINS-> Function`).
4. Delete-then-re-add of a single file's subtree, rather than rewriting the whole DB.
5. INHERITS extraction.
6. Multi-repo scoping by repository path.

CGC v0.3.1 behaviour fspec correctly did **not** copy: the global "relaxed" name fan-out, the `candidates[0]` guess, and the builtins skip applied to every language.

---

## 5. Bugs

Severity: **C** = critical, **H** = high, **M** = medium, **L** = low. All paths are under `~/projects/fspec/rust/`.

### B01 (C): The AST graph is bound to the daemon's cwd, not the session's project
- **Where:**
  - `graph/src/registry.rs:678-699`: `resolve_project_dir()` returns `std::env::current_dir()`.
  - `graph/src/registry.rs:541-562`: a singleton keyed only by `"ast-code"`.
  - `graph/src/ast_index.rs:45-80`: the index root also comes from cwd.
  - `agent-loop/src/graph_search_handler.rs:21`: `_session_id` is ignored.
  - Compare `agent-loop/src/agent_loop/handlers.rs:48-53`: SessionSearch receives `session.project`; GraphSearch receives nothing.
- **Scenario:**
  1. `fspec daemon` is started from `~/projects/fspec`.
  2. A session whose `project` is `~/projects/valheim-port` (or a worktree of fspec) calls `ast_index`.
  3. That indexes **fspec** and overwrites fspec's graph, and every `ast_search` in that session answers about the wrong repo.
  4. Once any session has opened the graph, it stays pinned to the first path for the life of the process.
- **Evidence on disk:** `~/.fspec.old/graph/ast-code.nano` exists (created 2026-04-03). An AST graph was created under `$HOME/.fspec/graph/`, which is the same directory as the global learnings graph, because the process cwd was `$HOME`.
- **Fix:**
  - Pass the session's canonical project root into `create_handler(project_root)`.
  - Key the registry by `(graph kind, canonical root)`.
  - Never read `current_dir()` in graph code.

### B02 (H): `ast_index path=<dir>` replaces the project graph with another tree
- **Where:** `graph/src/ast_index.rs:45-68` (custom root, gitignore off) → `:113` (the same `"ast-code"` DB) → `:174` (`load_entities_overwrite`).
- **Scenario:**
  1. The tool description invites `path: "tmp/my-repo"` to index an external repo.
  2. That **deletes every entity of the current project** and loads the external repo, with `File.path` relative to the *other* root.
  3. Later path-glob queries and the incremental mtimes refer to paths that don't exist under cwd, so the next `incremental: true` sees roughly 100% deleted and new files and falls back to full.
- **Fix:** one graph per indexed root, keyed by the canonical root (follows from B01), with the root stored on the graph and returned by every query.

### B03 (H): Incremental index ≠ full index (history-dependent edges)
- **Where:**
  - `graph/src/ast_pipeline/incremental.rs:551-582`: edges are reused if owned by an unchanged file.
  - `graph/src/ast_index.rs:248-317`.
  - The behaviour is *accepted* by KGRAPH-065 rule [5].
- **Scenarios:**
  - **(a) A new definition.** `a.py` (unchanged) does `from .b import helper; helper()`, but `b.py` had no `helper`, so the edge was pruned at full index. Add `def helper` to `b.py` and run incremental: only `b.py` is re-extracted, and `a.py::caller → b.py::helper` is **never** created until `a.py` is touched.
  - **(b) TS barrel resolution depends on `known_files`.** Add `utils/index.ts`, and every unchanged importer of `./utils` keeps its stale target.
  - **(c) A rename that keeps the name.** Delete `b.py::helper` and re-add it in `c.py`, changing `a.py`'s import: fine only if `a.py`'s mtime changed.
- **Rule violated:** determinism. The same tree gives different graphs depending on edit history.
- **Fix:**
  1. Split extraction into **per-file facts**: definitions, plus *unresolved* references (name, receiver/qualifier, import specifier, byte range). These are cacheable by content hash.
  2. Add a **global resolve pass** over all facts on every index. It is pure and in-memory, roughly O(refs).
  3. Only parsing is incremental; edges are always recomputed. This is better than CGC HEAD, which has the same gap.

### B04 (H): Same-named functions in one file are silently dropped, and their calls are misattributed
- **Where:**
  - `graph/src/ast_pipeline/ast_rust_extractor.rs:116-124` (`seen_names`; the first `fn new` wins).
  - `ast_ts_extractor.rs:117-123` (same pattern).
  - Identity `format!("{file_slug}::{name}")` at `helpers.rs:60`.
  - `extract_calls` (`ast_rust_extractor.rs:336`) uses `caller_slug = file::name`, so the *second* `new`'s body calls are attached to the *first* `new`.
- **Scenario:** `impl A { fn new() }` and `impl B { fn new() }` in one file: only `A::new` exists, and `B::new`'s callees appear as callees of `A::new`. The same happens to Java overloads, Python methods named `__init__`/`run` in several classes, and TS class methods.
- **Measured** (a regex count over fspec's 5,047 `.rs` files): **1,513 function definitions share a name with an earlier one in the same file, across 269 files.** All would be dropped.
- **Fix:**
  - Identity = `(path, container chain [impl/class/module], name, kind, start byte)` from the AST.
  - Display `qualifiedName` as `path::Type::method`.
  - Add a `HasMethod` / `ContainedIn` edge.

### B05 (M): `slugify_path` is not injective, so distinct files merge
- **Where:** `graph/src/ast_pipeline/helpers.rs:15-20` (`/`, `\` and `.` all become `-`) and `mod.rs:288-316` (dedup keeps the "richer" node).
- **Scenario:** `src/a-b.rs`, `src/a/b.rs` and `src/a.b.rs` all become `src-a-b-rs`.
  - One File node survives (with the other's `path` lost).
  - Their functions with equal names merge.
  - `is_owned_by_changed_file` attributes both files' entities to one.
  - `matches_entity_path_glob` resolves to the wrong path.
- **Fix:** key File by the normalized relative path itself, and key children by `(file path, …)`. Don't derive identity from a lossy slug.

### B06 (H): Extends/Implements edges are never produced, so `ast_hierarchy` is non-functional
- **Where:**
  - No extractor emits them: the only producer-side mention is the dedup type table at `graph/src/ast_pipeline/mod.rs:345-366`.
  - `graph/src/ast_hierarchy.rs:56-63` traverses them.
  - The test `graph/tests/ast_hierarchy_test.rs:49-51` **hand-writes** `{"edge":"Extends",…}` JSONL, so it passes vacuously.
  - KGRAPH-064 is marked done.
- **Scenario:** `class Dog(Animal)` (Python), `class A extends B` (TS/Java) or `impl Trait for T` (Rust), then `ast_hierarchy` returns `parents: [], children: []`.
- **Fix:** extract base and implemented types from the AST for each language (exact syntax facts), and resolve them in the global pass (B03). Replace the test with one that indexes a real fixture.

### B07 (M): `ast_hierarchy` "methods" are every function in the type's file
- **Where:** `graph/src/ast_hierarchy.rs:188-221`; the comment says "as an approximation".
- **Scenario:** a file with `struct A` and `struct B` lists B's methods and free functions as A's methods.
- **Rule violated:** "no guessing where an exact fact is available".
- **Fix:** record Type→Function containment from `impl`/class bodies.

### B08 (H): Calls are found by lexical scanning; most real calls are missed and some are wrong
- **Where:** `graph/src/ast_pipeline/edge_helpers.rs:78-186` (`extract_call_names_from_body`) and `:265-309` (`resolve_calls`).
- **Gaps:**
  - Ignores anything preceded by `.`, `->` or `::`. So Rust `Type::new()`, `module::f()` and `self.f()`, Python `self.f()`/`obj.f()`, TS `this.f()`/`obj.f()` and Java `obj.f()` produce **no** Calls edge.
  - Self-recursion is dropped (`:276-278`).
  - Nested functions: the outer function's body text includes the inner one, so the inner function's calls are attributed to both.
  - `#` is treated as a line comment for *every* language, so Rust `#[attr]` inside a body skips the rest of the line.
  - A `'` starts a "string", so Rust lifetimes (`&'static str`) inside a body swallow code up to the next `'`.
  - TypeRef vs Calls is decided by **uppercase first letter** (`:293-300`), which is a heuristic.
- **Effect:** for Rust, most call edges are absent, so `ast_callers`/`ast_callees`/`ast_call_chain` mostly return empty and `ast_dead_code` mostly returns false positives.
- **Fix:**
  - Record call sites from AST `call_expression` / `method_call` / `call` nodes with their callee expression (identifier, scoped path, receiver).
  - Resolve them in the global pass using exact facts: same scope, imports, module tree, `self` type from the enclosing `impl`.
  - Leave unresolvable calls as `UnresolvedCall` facts, not guesses.
  - Determine the kind (type vs function) from the resolved definition, not from letter case.

### B09 (M): Rust import resolution can't work in a Cargo workspace
- **Where:** `graph/src/ast_pipeline/ast_rust_extractor.rs:225-312`.
  - Line-based `use` parsing (`:230`): multi-line `use` and grouped `use a::{b, c}` fail.
  - Only `crate::`/`super::`/`self::`.
  - Candidates `{p}.rs`, `{p}/mod.rs`, `src/{p}.rs`, `src/{p}/mod.rs` relative to the **project root**.
  - `use crate::m::func` looks for `m/func.rs` rather than `m.rs` + item `func`.
  - `super::` is resolved as if it were `crate::`.
- **Scenario:** fspec's crates live at `rust/<crate>/src/...`. No candidate ever matches, so **zero** Rust Imports edges and zero cross-file Rust Calls for fspec itself. Workspace crate imports (`use codelet_graph::…`) are skipped by design.
- **Fix:**
  - Build the module tree from exact facts: crate roots from each `Cargo.toml` (`[lib]`/`[[bin]]` paths or defaults), `mod x;` declarations, and `#[path]`.
  - Resolve `use` trees from the AST `use_declaration`.
  - Map workspace crate names from the manifests.

### B10 (M): The ast-grep patterns miss generic, where-clause, const and unsafe Rust functions
- **Where:** `graph/src/ast_pipeline/ast_rust_extractor.rs:26-35`. The same pattern approach is used in every extractor.
- **Measured** with a `/tmp` probe using the same ast-grep 0.40.5 and patterns:
  - On a synthetic file, `fn generic<T>()`, `pub fn generic_pub<T>()`, `fn with_where<T>() where …`, `const fn` and `unsafe fn` were all missed.
  - On a 10% sample of fspec (503 files), **65 of 3,448 `function_item`s (1.9%) were missed**, all generics, e.g. `stream_loop.rs:42 pub async fn run_agent_stream<M, O>(`.
- **Fix:** select by node kind (`function_item`, `function_signature_item`, `impl_item`, …) in one tree walk (also P1).

### B11 (M): Dead-code output is unreliable, and CLEAN-006 depends on it
- **Where:**
  - Calls are incomplete (B08, B09).
  - `schemas/ast-queries.gq:215-222`: `unreferenced_types` says it checks "TypeRef, Extends, or Implements" but only checks `typeRef`.
  - `ast_dead_code.rs:96-104`: `count` is computed *after* `.take(max_results)`, so it reports the capped count.
  - `isTest = rel_path.contains("test")` (`ast_rust_extractor.rs:65`, `ast_c_extractor.rs:45`, `ast_cpp_extractor.rs:41`) marks `latest.rs`, `attestation.rs` and `contest/…` as tests, which hides their functions; meanwhile `#[cfg(test)] mod tests` inside a src file is *not* marked as test.
- **Scenario:** agents following CLEAN-006 would delete functions that are called via `Type::f()` or `self.f()`.
- **Fix:**
  - Block CLEAN-006 until B08/B09 land.
  - Return the uncapped count.
  - Detect tests from exact facts: `#[test]`/`#[cfg(test)]` attributes, Cargo `tests/` targets, language test conventions declared per language.

### B12 (M): Non-deterministic result order and truncation in `ast_callers`
- **Where:**
  - `graph/src/ast_call_chain/bfs.rs:162-180`: `reverse_adjacency` iterates a `HashMap` (random seed per process), so reverse lists come out in random order.
  - `find_all_reachable` (`:110-155`) is documented as "ordered … by slug within each depth level" but never sorts.
  - Truncation at `max_results` (`:140`) therefore returns a *different subset* each process.
  - `GraphSnapshot.known_slugs` is a `HashSet` (`snapshot.rs:16`).
- **Fix:** sort adjacency lists by slug when building them, and sort each BFS level before emitting and truncating.

### B13 (M): DeepSearch graph availability depends on process history
- **Where:** `agent-loop/src/deep_search_handler.rs:200-204` uses `is_graph_initialized(...)`, i.e. whether *something in this process already opened* a graph.
- **Log:** `~/.fspec.old/logs/fspec-combined.log.2026-07-06`: `2026-07-06T21:17:19.985479Z WARN codelet_agent_loop::deep_search_handler: [deep-search-dispatch] ENTER provider=openai model=Some("qwen") … project_path=/home/rquast/projects/fspec can_recurse=true graph_available=false`.
- **Fix:** decide from the session project's on-disk graph state (exists and schema matches), not from in-memory registry state.

### B14 (M): mtime change detection has a TOCTOU gap and blind spots
- **Where:**
  - `graph/src/ast_index.rs:139-161`: files are extracted first, then the tree is **walked again** and mtimes stamped.
  - `incremental.rs:440-461`.
- **Scenarios:**
  - A file saved between its extraction and the later `stat` is stored with the *new* mtime and the *old* content, so it is never re-extracted.
  - Tools that preserve mtimes (`rsync -t`, `tar x`, `cp -p`) change content with no mtime change, so it is missed.
  - A file present in the second walk but not the first gets an mtime but no entities.
- **Fix:** hash the exact bytes that were parsed (sha2 is already a dependency) and store `contentHash` on File; compare hashes. Walk once.

### B15 (M): Windows paths: File.path keeps `\`, so incremental never matches
- **Where:** `graph/src/ast_pipeline/mod.rs:118-122`. `rel_path` is `to_string_lossy()` without normalization, while `collect_file_mtimes` (`incremental.rs:446-450`) and `known_files` (`mod.rs:255-262`) use `/`.
- **Scenario:** on Windows, `stamp_file_mtimes` never finds `path`, so the stored mtimes are empty, so every incremental run silently falls back to full. Path globs and import resolution also compare `\` against `/`.
- **Fix:** normalize to `/` once at the walk and pass only normalized relative paths downstream.

### B16 (M): Silent fallbacks and swallowed errors (against "no fallbacks")
- **Where:**
  - Incremental silently runs a full index when stored mtimes can't be read (`ast_index.rs:209-215`), when there is no prior index or more than 50% changed (`:222-232`), and when export fails (`:257-262`).
  - Query errors are dropped:
    - `build_adjacency_list` `if let Ok` (`ast_call_chain/mod.rs:108`);
    - `build_file_path_index` (`ast_dispatch.rs:551`);
    - `find_type` `.ok()?` (`ast_hierarchy.rs:99-101`);
    - `query_single_hop` and `find_methods_in_file` return `Vec::new()` on error (`ast_hierarchy.rs:176-221`);
    - `GraphSnapshot::load` ignores errors (`snapshot.rs:31-50`).
  - Per-file extraction failures are only `warn!`ed (`mod.rs:268-273`) and the index reports success.
  - Dependency extractors fail "non-fatal" every run for fspec, which has no root `Cargo.toml` (`ast_index.rs:414-420`).
- **Effect:** a broken graph looks like "no callers".
- **Fix:** propagate errors; include `files_failed: [{path, error}]` in the index result; delete the 50% heuristic (the cost model is fixed by P4 anyway).

### B17 (M): Concurrency: init race, reset under readers, no cross-process writer lock
- **Where:**
  - `graph/src/registry.rs:541-562`: check, then `init_graph().await`, then insert. Two first callers both `Database::init` the same path, and the loser's handle is orphaned.
  - `registry.rs:600-613` and `ast_index.rs:83-111`: `reset` does `remove_dir_all` while other sessions hold `GraphDatabase` clones.
  - nanograph has only an in-process writer mutex. Two fspec processes on the same project (the log shows combined mode bootstrapping on `/home/rquast/projects/fspec` at `2026-11-03T22:45:21` and again at `22:47:32`, and three times on 2026-11-06) can Overwrite-load the same `.nano` directory concurrently.
- **Fix:**
  - Per-root `tokio::sync::OnceCell` init.
  - A `RwLock` per graph, with reset and index taking the write side.
  - An OS file lock (`.nano/.fspec-writer.lock`) for cross-process single-writer, failing loudly rather than racing.

### B18 (L): Calls edge metadata is declared but never populated
- **Where:** `schemas/ast-code.pg` (`Calls { callCount, isConditional }`). No producer anywhere (grep). `function_calls` doesn't even return them (`ast-queries.gq:103-109`), yet `ast_call_chain` reports `call_details[].callCount/isConditional` (`ast_call_chain/mod.rs:116-119, 171-176`), which are always null.
- **Fix:** populate them from AST call sites (count per caller/callee; inside if/match/loop), or remove them from the schema and output.

### B19 (L): Dependency extraction misses workspace layouts and collides across ecosystems
- **Where:**
  - `cargo_dep_extractor.rs:17,41`: only `<root>/Cargo.toml`; workspace `members` are joined literally, so glob members such as `crates/*` are unsupported.
  - Dependency slug `dep::{name}` (`helpers.rs:263`): npm `uuid` and crate `uuid` merge into one node.
- **Fix:** discover manifests during the walk; key Dependency by `(ecosystem, name)`.

### B20 (L): `find_paths` can blow up in memory
- **Where:** `graph/src/ast_call_chain/bfs.rs:74-91`. Revisits at equal depth are allowed and every enqueue clones the whole path, so the queue size equals the number of shortest partial paths, which grows exponentially in diamond-shaped graphs. The only bound is `max_chains` *found*.
- **Fix:** BFS once to compute distance plus a parents multimap, then enumerate up to K paths from the target backwards.

### B21 (L): Cyclomatic complexity is text heuristics
- **Where:** `graph/src/ast_pipeline/complexity.rs:20-24`. In Rust, `||` closures count as decision points, `loop` counts, `?` doesn't, and `=>` counts match arms including the ones inside macros.
- **Fix:** count decision nodes in the AST per language (exact).

### B22 (L): Walker gaps
- Symlinked files are skipped silently: `file_type().is_file()` is false for symlinks (`mod.rs:210`).
- There is no file-size cap: a minified bundle gets scanned by every pattern, and pattern cost is superlinear (P1).
- `.h` is parsed as C or C++ by content guess (`mod.rs:141-148`).
- **Fix:** an explicit symlink policy (resolve inside the root, report outside); a size cap reported in `files_skipped`; `.h` language declared in config rather than guessed.

### B23 (L): Dead schema elements
- The `Module` node is never produced.
- `Function.qualifiedName` always equals `slug`.
- `Calls` properties (see B18).

### Spec drift
| Card / feature | Claims | Reality |
|---|---|---|
| KGRAPH-065 title | "Live File Watching" | No watcher, no auto-trigger; no `notify` dependency in `codelet-graph`. |
| KGRAPH-065 rule [4] / `incremental-reindexing.feature:83` | Fall back to full above 50% | A fallback, which violates the product's "no fallbacks" rule. |
| KGRAPH-059 architecture note [0] | "Nanograph does NOT support variable-length path traversal" | Bounded `$a calls{1,5} $b` exists in 1.0.0 (`docs/user/queries.md:121-129`). §9 has a measurement. |
| KGRAPH-064 / `class-hierarchy…feature` | Parents, children and interfaces via Extends/Implements | Never extracted (B06); the test feeds hand-made edges. |
| KGRAPH-014 | "Project-scoped (one per working directory)" | Effectively one per *process* cwd (B01). |
| KGRAPH-014 research attachment | Function key = qualified module path (`graph::dispatch::handle_search`) | `file_slug::name` (B04, B05). |
| KGRAPH-062 (review-findings) | "Rule[4] wording vs implementation (text-based, not tree-sitter)" | Known drift, never fixed (B21). |
| `ast-queries.gq:213-215` comment | `unreferenced_types` checks TypeRef, Extends and Implements | Checks TypeRef only (B11). |
| `bfs.rs:111` doc | Sorted by slug within depth | Not sorted (B12). |

### Tests that pass vacuously
- `graph/tests/ast_hierarchy_test.rs`: hand-written Extends/Implements JSONL; the pipeline never produces those edges.
- The `ast_call_chain` tests assert the presence of `call_details` but cannot detect that `callCount`/`isConditional` are always null.
- `rust/tools/src/graph_search/tests.rs:33-34`: the AstIndex mock returns `entities_loaded: 0`. This only tests deserialization.
- No test indexes a **Cargo workspace** or a real multi-file Rust crate end to end and asserts cross-file Calls edges. That is why B09 is invisible.
- No test checks that incremental equals full on the same final tree (B03).

---

## 6. Performance improvements

Measurements were taken on this machine (20 cores) in `/tmp` only:
- `/tmp/asst/probe`: ast-grep 0.40.5, the same version as fspec's lockfile.
- `/tmp/asst/nbench`: nanograph 1.0.0, the same as fspec, with fspec's real `ast-code.pg` and `ast-queries.gq`.

| Measurement | Value |
|---|---|
| fspec indexable files (git-tracked, after SKIP_DIRS) | 5,041 `.rs`, 30 `.ts`, 22 `.py`, plus a handful of others. Rust source is 41 MB. |
| tree-sitter parse of all 5,036 tracked `.rs` files (one parse each) | **2.23 s** |
| One kind-based DFS over all those trees (collecting `function_item`) | **0.32 s** |
| The extractor's 14 fn+type pattern scans, run once over all files | **551.9 s** (about 7.8 ms per pattern per file; slowest file 291 ms for 8 patterns) |
| Rust extractor work per file today | 4 parses (`ast_rust_extractor.rs:71,326,370`, `variables.rs:367`) plus about 30 pattern scans (8 fn + 6 type + 8 calls + 8 type-refs) plus variable patterns |
| **Estimated full `ast_index` of fspec today** | **about 20 minutes, single-threaded** (30/14 × 552 s ≈ 1,180 s), before load |
| nanograph 1.0.0 `function_calls` single query (2k / 10k / 35k functions) | 4.1 / 4.9 / 6.7 ms |
| → `build_adjacency_list` (one query per function) per `ast_callers` / `ast_callees` / `ast_call_chain` call | **8.3 s / 49 s / 235 s** (estimated as N × the measured single-query time) |
| One query returning every Calls edge (`$a calls $b`) | **5.2 / 16.9 / 54.6 ms** |
| Native bounded traversal `$a calls{1,5} $b` (1.0.0) | 5.4 / 6.0 / 7.6 ms |
| `all_functions` (what every `ast_search` runs), with 800 B of source per function | 10.7 / 34 / 103 ms, producing **2.1 / 10.6 / 37 MB of JSON** per call, for the Function type alone |
| `.gq` re-parse inside each `run_json` | 0.145 ms (negligible unless done N times) |
| Overwrite load of the whole graph (2k / 10k / 35k functions) | 0.12 / 0.35 / 1.16 s. On-disk size 4.8 / 23 / 81 MB. Five repeated Overwrites did **not** grow the directory. |
| `Database::open` of the 35k graph, then the first query | 65 ms, then 15 ms; RSS 141 MB (1.0.0 loads the whole graph into memory) |

### P1: One parse, one AST walk, data-driven per language (the biggest win)
- **Evidence:**
  - Pattern scanning costs about 1,700x a DFS: 551.9 s vs 0.32 s for the same trees.
  - Each Rust file is parsed 4 times and scanned about 30 times.
  - Every extractor uses the same `find_all(pattern)` approach (15 `build_function_node` call sites).
- **Change:** parse each file once and walk the tree once. Collect definitions, containers, call sites, imports and base types by node kind, using per-language **data** (node-kind tables, or tree-sitter `tags.scm`-style queries shipped as config) instead of 14 hand-written extractors. This also fixes B04, B08 and B10, and keeps the engine language-agnostic: languages are declared, not coded.
- **Impact:** full extraction of fspec drops from about 20 min to about **3 s** single-threaded (2.2 s parse plus a walk), roughly **400x**.
- **Effort:** L. This rewrites the extractor layer; dependency extractors are unaffected.

### P2: Load the call graph in one query (or use native traversal)
- **Evidence:** `ast_call_chain/mod.rs:100-133` issues one `function_calls` query per function. Measured cost is 49 s (10k functions) to 235 s (35k functions) per request, against 17–55 ms for a single all-edges query.
- **Change:** add `all_calls()` returning `(from.slug, to.slug)`; build forward and reverse adjacency once per graph version; cache it on `GraphDatabase` and invalidate it on load. For neighbours up to depth N use `calls{1,N}`.
- **Impact:** **about 1,000–4,000x** per transitive query. ast_callers on a fspec-sized graph goes from minutes to under 100 ms.
- **Effort:** S.

### P3: Stop materializing every function's source on each `ast_search`
- **Evidence:** `ast_dispatch.rs:631-692` runs `all_functions` / `all_types` / … for every search; `all_functions` returns `source` (`ast-queries.gq:19-22`). Measured 103 ms and 37 MB of JSON per call at 35k functions, and `ast_complexity`, `GraphSnapshot` and dead-code do the same.
- **Change:**
  - Add a name-only projection query for `search_mode=name`.
  - Keep an in-memory symbol index (name → slugs, path → slugs) per graph version.
  - Fetch `source` only for the result rows.
  - `ast_complexity` should use a projection without source.
- **Impact:** about **50–100x** lower latency and allocation on searches at fspec scale.
- **Effort:** S.

### P4: Make incremental truly incremental: content-hash fact cache plus one resolve plus one load
- **Evidence:**
  - Incremental runs `export_all_entities()` twice (`incremental.rs:467-468` via `read_stored_mtimes`, and `ast_index.rs:257`), decoding every Arrow row including source to JSON.
  - It then Overwrite-loads the whole graph (`:317`).
  - Full index walks the tree twice (`ast_index.rs:141,159`).
- **Change:**
  1. Walk once.
  2. Hash each file (sha2 is already a dependency; hashing 41 MB is about 0.1–0.2 s).
  3. Look up the per-file *fact* cache (definitions plus unresolved references, as in B03) by hash; parse only misses.
  4. Run the global resolve pass.
  5. Do a single Overwrite load. At fspec scale that is about 1.2 s and deterministic, so no per-entity diffing is needed.
- **Impact:** a one-file edit goes from "export ×2 + extract + overwrite" to roughly 0.2 s hash + resolve + 1.2 s load, with output identical to a full index. Removes the 50% fallback.
- **Effort:** M.

### P5: Parallel extraction
- **Evidence:** `ast_pipeline/mod.rs:267-274` is a sequential loop.
- **Change:** rayon `par_iter` over files. Sort the output by path for deterministic order before resolve.
- **Impact:** about 10–15x on 20 cores for the parse phase after P1; today it would cut about 20 min to about 2 min if done alone.
- **Effort:** S.

### P6: `ast_neighbors` runs 14 queries
- **Evidence:** `ast_dispatch.rs:715-755` runs all 14 single-hop queries for any node, at about 5 ms each.
- **Change:** resolve the node's type first and run only its applicable queries, or answer from the cached adjacency (P2).
- **Impact:** 3–5x.
- **Effort:** S.

### P7: Memory and startup
- **Evidence:**
  - nanograph 1.0.0 keeps the whole graph resident (141 MB RSS for the 35k synthetic graph).
  - The registry never closes graphs (`registry.rs:524-527`).
  - After the B01 fix there will be one graph per project root.
- **Change:**
  - Upgrade to nanograph 1.3.0 (lazy, metadata-first open; §9).
  - Close idle per-root graphs.
  - Consider not storing `source` in the graph: store `(path, byte range, file contentHash)` and read the bytes from disk on demand. That is an exact fact, and the hash tells you when the file has changed.
- **Impact:** 2–5x smaller DB (source dominates row size) and bounded daemon memory.
- **Effort:** S (close idle graphs) / M (source on demand).

### P8: Don't block runtime workers during indexing
- **Evidence:** `graph_search_handler.rs:21-25` uses `block_in_place` + `block_on`, and `dispatch_full` runs CPU-bound extraction inline (an estimated 20 min on fspec today).
- **Change:** `spawn_blocking` for walk and extract; single-flight indexing per root (a second caller awaits the same job); report progress.
- **Impact:** avoids runtime starvation with several sessions.
- **Effort:** S.

### P9: Freshness without a watcher storm
- **Evidence:** there is no watcher today. CGC v0.3.1's watcher re-parses the whole repo on every save, O(repo) per save.
- **Change:** after P1 and P4 a full-equivalent incremental run is seconds, so:
  - store `indexed_at` and the per-file hashes;
  - have every query response report `stale_files` (walk + stat + hash compared, about 0.2 s), or
  - add an optional `notify` watcher that only marks files dirty with a debounce and coalesces into one single-flight index job.
- **Impact:** fresh results without per-save full work.
- **Effort:** M.

### P10: Things the reference does faster
- CGC HEAD batches writes (UNWIND) and builds a precomputed name index for linear-time resolution (`09b6db79`, 257.6 s → 6.5 s). fspec already batches writes (one JSONL load); the equivalent fspec win is the global resolve pass over an in-memory symbol table (B03/P4).
- CGC HEAD parses in parallel with 10 workers (P5).
- CGC indexes `(name, path)` for MATCH seeks. fspec's equivalent is the cached adjacency and symbol index (P2/P3) and, with nanograph 1.3.0, `fast_search` on `@key`.

---

## 7. Log evidence

- **Recent logs** (`~/.fspec/logs/private/fspec-combined.log.2026-11-03` … `2026-11-06`): **zero** graph-related lines. `grep -ic "graph|KGRAPH|nanograph|ast_index|lance"` gives 0, including the 1.06 MB file for 2026-11-06. In the last week nobody called GraphSearch, and no graph error, slow-operation or warning line was logged. `client.log.2026-10-28` also has 0.
- **Process/workspace evidence relevant to B01 and B17:**
  - `2026-11-03T22:45:21.834634173+00:00 INFO fspec::combined fspec combined mode bootstrapping port=33739 workspace=/home/rquast/projects/fspec`
  - `2026-11-03T22:47:32.065718555+00:00 INFO fspec::combined fspec combined mode bootstrapping port=36969 workspace=/home/rquast/projects/fspec`. A second process started on the same workspace two minutes later; both would share `.fspec/graph/ast-code.nano`, with no cross-process lock.
  - `2026-11-04T07:22:00.266607714+00:00 … workspace=/home/rquast/projects/valheim-android` and `2026-11-04T07:22:31.511325810+00:00 … workspace=/home/rquast/projects/valheim-port`. Several projects are in use; any session whose project differs from its process cwd hits B01.
  - 2026-11-06: three bootstraps on `/home/rquast/projects/fspec` (00:40:22, 00:57:46, 02:16:39).
- **Older logs** (`~/.fspec.old/logs/fspec-combined.log.2026-07-06`):
  - `2026-07-06T21:17:19.985479Z WARN codelet_agent_loop::deep_search_handler: [deep-search-dispatch] ENTER provider=openai model=Some("qwen") … project_path=/home/rquast/projects/fspec can_recurse=true graph_available=false`. DeepSearch ran without GraphSearch because no graph had been opened in-process yet (B13).
- **On-disk evidence:** `~/.fspec.old/graph/ast-code.nano/` (48 KB, 2026-04-03: `schema.pg`, `schema.ir.json`, `graph.manifest.json`, `nodes/`, `edges/`). A project-scoped AST graph was created in the *home* `.fspec/graph`, the same directory as the global learnings graph, because the process cwd was `$HOME` (B01). The fspec repo itself has **no** `.fspec/graph/` directory; the code graph has never been built for fspec.
- **Conclusion:** the logs show no failures because the feature is effectively unused. The bugs above come from code reading and measurements, not from runtime errors.

---

## 8. Recommended plan (draft cards)

Order: 1 → 2 → 3 → 4 → 5. Card 6 can run in parallel with 1; card 7 any time. Each card must keep fspec language-agnostic (languages declared as data), deterministic, free of fallbacks, and exact.

1. **CG: Scope the code graph to the session's project root**
   - **Description:** GraphSearch reads `std::env::current_dir()` and a process-global `"ast-code"` singleton, so sessions in other projects or worktrees index and query the daemon's cwd (B01). `ast_index path=` also overwrites the project graph (B02). Pass the session's canonical project root into the handler, key the registry by `(kind, canonical root)`, store the root on the graph and return it in results. Make DeepSearch availability depend on the project's on-disk graph state (B13). Add per-root single-flight init, a reset/index write lock and a cross-process writer lock file (B17).
   - **Rules:**
     - No `current_dir()` in graph code.
     - One graph per canonical root.
     - A custom path indexes into its own graph, never the project's.
     - A second writer process fails with an explicit error.
     - Availability is a pure function of on-disk state.

2. **CG: Single-pass AST fact extraction (data-driven per language)**
   - **Description:** Replace the 14 ast-grep pattern extractors with one engine. It parses each file once, walks the tree once, and emits per-file *facts*: definitions with their container chain and byte span, imports as written, call sites with their callee expression, base and implemented types, and test markers from attributes. Per-language behaviour comes from declared node-kind/query config. This fixes B04, B08, B10, B21 and B22, and delivers P1 and P5. Identity becomes `(normalized path, container chain, name, kind, start byte)`; File is keyed by normalized path (B05, B15).
   - **Rules:**
     - No text scanning of bodies.
     - No case-based or other heuristics.
     - Every definition node in the tree is captured (tested by counting node kinds against facts).
     - Same-named definitions in different containers are distinct.
     - Output is sorted by path, then span.
     - Paths use `/`.

3. **CG: Global deterministic resolve pass and exact Rust module resolution**
   - **Description:** After all facts exist, resolve imports, calls, type refs and inheritance in one in-memory pass, using exact facts: the module tree built from manifests and `mod` declarations (Cargo crate roots, workspace crate names), import tables, and the `self` type from the enclosing impl/class. Unresolvable references stay as explicit `Unresolved*` facts with a reason, not guessed edges. Produces Extends/Implements and Type→method containment, fixing B06, B07 and B09. Also populate or remove `callCount`/`isConditional` (B18) and the `Module` node (B23).
   - **Rules:**
     - The same tree always yields byte-identical graph output.
     - Ambiguous targets are never guessed.
     - Unresolved counts are reported per kind.
     - End-to-end tests run on a Cargo workspace fixture and assert cross-crate Calls.

4. **CG: Content-hash incremental indexing equal to full indexing**
   - **Description:** Walk once and hash each file's bytes. Reuse cached facts by hash and parse only the files whose hash changed. Always rerun the global resolve, then a single Overwrite load. Remove the mtime logic, the 50% fallback and the double `export_all_entities` (B03, B14, B16; P4). Index results report `files_parsed`, `files_reused`, `files_failed[{path,error}]`, `files_skipped[{path,reason}]` and `root`.
   - **Rules:**
     - Incremental output == full output on the same tree (property test).
     - No fallbacks.
     - Per-file failures are surfaced, never only logged.
     - Hashes come from the bytes that were parsed.

5. **CG: Query layer performance and determinism**
   - **Description:** Cache the forward and reverse call adjacency and a symbol index per graph version, loaded by one `all_calls` query and projection queries (P2, P3, P6). Use native `calls{1,N}` where only reachability is needed. Fetch source only for result rows. Sort adjacency and BFS levels by slug (B12). Replace path enumeration with distance-plus-parents (B20). Return uncapped totals alongside capped item lists (B11). Propagate query errors instead of returning empty results (B16).
   - **Rules:**
     - No per-function query loops.
     - Results and truncation are identical across processes.
     - An error is never presented as "no results".
     - Every response includes `graph_version` / `indexed_at`.

6. **CG: Upgrade nanograph 1.0.0 → 1.3.0**
   - **Description:** Bump `nanograph` to 1.3.0 and `arrow-*` to 58 (or drop them). Replace the removed `Database::snapshot()` usage: `stats()` moves to the manifest metadata API; `export_all_entities()` moves to `store::export::build_export_rows_at_path`. Detect legacy v3 storage on open: rebuild the AST graph and migrate the learnings graph explicitly with `migrate_storage_to_lineage_native`. Audit JSONL producers for unknown properties (1.2.2 rejects them). Add a smoke test for the `contains` edge name. Details in §9.
   - **Rules:**
     - No silent auto-migration of user data. The learnings migration is explicit, and a backup is kept.
     - The AST graph is derived data and is rebuilt.
     - The upgrade lands with tests for open, legacy detection, stats and export.

7. **CG: Fix spec drift and vacuous tests in KGRAPH features**
   - **Description:**
     - Retitle KGRAPH-065 (no watcher exists) and drop its 50% fallback rule.
     - Correct the KGRAPH-059 note on variable-length traversal.
     - Make the hierarchy, call-chain-metadata and dead-code tests index real fixtures instead of hand-written JSONL.
     - Block CLEAN-006 until cards 2–3 are done.
     - Optionally add a debounced dirty-marking watcher with single-flight indexing (P9) once card 4 makes indexing cheap.
   - **Rules:**
     - Every scenario's Given builds the graph through `ast_index`, not hand-made edges.
     - Feature text matches behaviour.
     - Dead-code output carries an explicit unresolved-reference count.

---

## 9. nanograph 1.0.0 → 1.3.0

- **Refs:** v1.0.0 = `67b8c68c4bf08ec5dd34708290250f05251434cc`, v1.3.0 = `d0223f10b6b4096b61618056ec97760cfe989b81`. 46 commits apart. There is no CHANGELOG; the release notes are `docs/dev/release-notes-v1.1.1.md` … `v1.3.0.md`.
- **Users in the workspace:** `codelet-graph` (`rust/graph/Cargo.toml:17`) is the only nanograph user. Per `Cargo.lock`, nanograph is also the only user of arrow, datafusion, lance and tantivy, apart from codelet-graph's own `arrow-array`/`arrow-schema = "57"` pins (`rust/graph/Cargo.toml:43-44`).
- **Verdict:** not a drop-in bump. There are three hard breaks:
  1. `Database::snapshot()` is removed. Verified: present at v1.0.0 `crates/nanograph/src/store/database.rs:531`, absent at v1.3.0.
  2. Arrow 57 → 58.
  3. 1.0.0 databases will not open under 1.3.0 without migration.
- The load and query API fspec uses is unchanged.

### 9.1 Breaking API changes affecting codelet-graph
| API | fspec call site | 1.3.0 |
|---|---|---|
| `Database::init` / `Database::open` | `graph/src/database.rs:37`, `:51` | Signatures unchanged. Init creates new "NamespaceLineage" storage. Open **rejects legacy v3 storage** (§9.2). |
| `Database::load` (Merge when `@key` exists) | `database.rs:146`, used from `lib.rs:83`, `bundle.rs:138` | Unchanged. |
| `load_with_mode(.., LoadMode::Overwrite)` | `database.rs:159`, used from `ast_index.rs:174,317`, `bundle.rs:142` | Unchanged (v1.3.0 `store/database/persist.rs:90`). |
| `run_json(.., JsonParamMode::Standard)`, `RunResult`, `to_rust_json()`, `affected_nodes/edges` | `database.rs:190-199` | Unchanged. `MutationResult` gains `matched_nodes` (additive). |
| `catalog()` node/edge types and properties | `database.rs:207-230, 267-285` | Unchanged (additive fields `key_property`, `media_uri_props`). |
| **`snapshot()` → `GraphStorage.node_segments/edge_segments/batches`, `edge_batch_for_save`** | `database.rs:236-260` (`stats`), `bundle.rs:154-300` (`export_all_entities`, hand-rolled Arrow decoding) | **Removed** in `dcc90df` (1.1.x); `store::graph` became `pub(crate)`. Replacements: `stats()` → `store::metadata::DatabaseMetadata::open(path)?.manifest().datasets[*].row_count`; `export_all_entities()` → `store::export::build_export_rows_at_path(db_path, false, false)` (v1.3.0 `store/export.rs:13`). The export function is **async** and already returns fspec's `{"type","data"}` / `{"edge","from","to","data"}` shape keyed by `@key`, so most of `bundle.rs` disappears. The async change ripples to `ast_index.rs:209,257`, `incremental.rs:467`, `bundle.rs:33`. |
| `.gq` grammar | `schemas/ast-queries.gq` | Additive only: `put`, `where {}` blocks in mutations, `is null`, and a `contains` filter on list properties. **Watch item:** fspec has an edge named `contains` (`$f contains $fn`). The grammar tries traversal before filter, so it should still parse, but add a smoke test. |
| `.pg` grammar | `schemas/*.pg` | No change. |

### 9.2 Behaviour and on-disk format
- **Storage generation (breaking).**
  - 1.0.0 writes legacy v3: `graph.manifest.json` + `_wal.jsonl` + Lance 3 datasets.
  - From 1.2.0 (`db420d7`, `d7992fa`, `dff98c6`), new databases are NamespaceLineage (`storage.generation.json`, `__graph_snapshot`, `__graph_tx`, `__graph_deletes`, `__blob_store`).
  - Opening a 1.0.0 database fails with "uses legacy v3 storage; run nanograph storage migrate … --target lineage-native" (`store/metadata.rs:41`).
  - The migration API is `store::storage_migrate::migrate_storage_to_lineage_native(db_path)` (v1.3.0 `storage_migrate.rs:206`). It keeps `<db>.v3-backup`, refuses to run if that already exists, and drops CDC and version history.
  - fspec's `open_or_init*` (`database.rs:274-314`) only checks for `schema.ir.json`, which still exists, so it must detect legacy storage explicitly:
    - AST graph: delete and rebuild (derived data).
    - Learnings graph: explicit migrate, with the backup kept (user data).
- **Lance 3 → 6:** the format is compatible for lineage databases; upstream verified that v4 databases open with the v6 binary.
- **Stricter JSONL loader (1.2.2, `90bcf06`):** unknown property names now fail the load with a row number. Before, they were dropped silently. Audit every `GraphEntity` producer against `ast-code.pg` / `learnings.pg`. That audit is a good thing: it would have caught dead or mistyped properties.
- **Edges to missing `@key` nodes:** still a hard error, so fspec's pruning (`ast_pipeline/mod.rs:288-415`) stays. With the resolve-pass design (card 3), dangling edges simply should not be generated.
- **Concurrency:** still no cross-process lock. 1.3.0 commits through Lance optimistic concurrency with `conflict_retries(0)` (`lance_io.rs:497`, `namespace.rs:532`), so a racing second process gets a conflict error instead of a silent clobber. That partly mitigates B17, but fspec should still take its own writer lock.
- **Schema migration** now blocks unsafe `@key`/`@unique`/enum changes. This does not affect fspec, which rebuilds on a schema-hash change.

### 9.3 Upstream fixes relevant to the bugs above
- **`5d6cbcf`:** fixes the "number of columns(N) must match number of fields(M)" crash when a `not {}` block traverses into an outer-bound variable, or two traversals land on the same variable. This makes it safe to add the missing `not { $x extends $t }` / `not { $x implements $t }` arms to `unreferenced_types` (B11).
- **`d3191c8` (1.2.1):** edge delete by endpoint used to hardcode a `name` column. fspec keys on `slug`, so it was broken for fspec. Fixed; a missing endpoint is now a no-op. Relevant if per-file deletes are ever used.
- **`661f163`:** `SourceDedupeBehavior::FirstSeen` on merge-insert prevents Lance "Ambiguous merge inserts" when a batch has duplicate keys. Relevant to `@key` Merge loads (learnings, bundle import).
- **`90bcf06`:** unknown-property rejection (above).
- **1.2.1:** database paths containing spaces, `#` or `%` fixed. Relevant to project roots with such characters once graphs are per-root (B01).
- **`dcc90df` / `82edb35` (1.1.x):** metadata-first runtime. `open` no longer loads the whole graph into memory (1.0.0 kept it in `RwLock<Arc<GraphStorage>>`), and writes persist only the datasets they touch.

### 9.4 Performance improvements for free
- Lazy open and touched-dataset writes reduce daemon memory (P7). In 1.0.0, 141 MB RSS was measured for a 35k-function graph.
- A compiled-query cache keyed by query fingerprint (`661f163`) removes the per-call `.gq` re-parse (0.145 ms each, measured).
- `fast_search()` for single-equality lookups on an indexed `@key` makes `Function { slug: $slug }` lookups cheaper. Today they cost 4–7 ms each (measured), mostly a scan.
- DataFusion runs relational tails (filter, project, order, limit); upstream `CrossJoinExec`.
- Lance 6: segmented inverted index for FTS, much faster range queries, SIMD kernels.
- **Already in 1.0.0 and unused:** bounded variable-length traversal `$a calls{1,N} $b` (v1.0.0 `docs/user/queries.md:126`). It measured 5–8 ms for depth 5 on a 35k-function graph. Unbounded `{1,}` is still rejected in 1.3.0, and neither version returns paths or depths, so `ast_call_chain` keeps a Rust BFS, but over a cached adjacency (P2).

### 9.5 Dependency effects (see the note at `rust/Cargo.toml:376-378`)
| Crate | with nanograph 1.0.0 | with nanograph 1.3.0 | fspec lock today |
|---|---|---|---|
| arrow / arrow-array | 57.3.0 | **58.3.0** | 57.3.0 |
| datafusion | 52.3.0 | **53.1.0** | 52.3.0 |
| lance / lance-index | 3.0.x | **6.0.0** (adds lance-io, lance-namespace(-impls), lance-table) | 3.0.1 |
| tantivy | 0.24.2 | **removed** (Lance 6 uses `lance-tokenizer`) | 0.24.2 |
| object_store | 0.12.5 | 0.12.5 **and** 0.13.2 (duplicated) | 0.12.5 |
| Packages in nanograph's lock | 566 | 635 | |

- `rust/graph/Cargo.toml:43-44` must become `arrow-* = "58"`, or be dropped entirely once `bundle.rs` uses `build_export_rows_at_path`.
- The `rust/Cargo.toml:378` comment ("nanograph + arrow + datafusion + lance + tantivy") becomes stale: tantivy leaves, and lance-namespace plus a second object_store arrive.
- The test-binary disk-bloat concern in that note stays, and grows slightly with the larger dependency set.
- Toolchain: nanograph's own pin moves 1.91.0 → 1.94.1. fspec pins 1.95.0, so there is no conflict. Lance 6 / DataFusion 53 minimum Rust versions were not verified, because they are not in the local registry cache.

### 9.6 New 1.1–1.3 features that help fspec
- `put Type {...}` single-statement upsert by `@key`, and `update … where {…; x is null}` compare-and-swap. Useful for learnings counters without full JSONL Merges.
- Fixed `delete … where` by `@key` endpoints. Per-file deletes become *possible*, but the recommended design (card 4) keeps a deterministic whole-graph Overwrite, which measured 1.2 s at fspec scale.
- Lineage CDC (`changes()`), which could drive cache invalidation of the adjacency and symbol index (P2/P3).
- `nanograph lint` for `.gq` files against a schema. Wire it into tests so query/schema drift (like `callCount` not being returned) is caught.

---

## Appendix: reproduction artifacts (all in /tmp)
- `/tmp/asst/probe`: ast-grep 0.40.5 probe.
  - `target/release/probe <file-list>` reports parse vs pattern-scan vs DFS time and missed `function_item`s.
  - `sample.rs` is the synthetic coverage case.
  - `rs-files.txt` / `rs-sample.txt` are the file lists used.
- `/tmp/asst/nbench`: nanograph 1.0.0 benchmark using copies of fspec's `ast-code.pg` / `ast-queries.gq`, plus two bench queries.
  - `target/release/nbench <n_functions>`.
  - `REPEAT=1` measures on-disk growth over repeated Overwrites.
  - `OPEN_ONLY=1` measures open time and RSS.
  - Benchmark DB directories were deleted after the runs. The build dirs are 1.8 GB (nbench) and 313 MB (probe), deletable.
- Clones are in `/tmp/graph-reference/` (223 MB).
