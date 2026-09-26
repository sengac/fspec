# CONFIG-009 Research — Config-Scope Key Ownership: User vs Project

**Status:** Research complete (discovery phase, CONFIG-009)
**Date:** 2026-09-26
**Companion to:** BUG-193 (root-cause write-path fix). This doc codifies the **design rule** that
BUG-193 enforces on the write side, plus a candidate validation surface so future contamination is
detectable.

---

## 1. Question

Which top-level keys of `fspec-config.json` belong in the **user** scope
(`~/.fspec/fspec-config.json`) vs the **project** scope (`spec/fspec-config.json`)? Today the
boundary is implicit — it lives only in code comments and in which reader/writer touches which
scope. There is no spec-level rule, no registry, and no validation. This task defines the rule
and (optionally) a surface that surfaces violations.

## 2. Key inventory (from code, not from memory)

Legend: **U** = user-scope writer exists · **P** = project-scope writer exists ·
**rU** = read from user scope · **rP** = read from project scope.

| Key | Writers | Readers | Correct scope | Notes |
|---|---|---|---|---|
| `providers.openai.profiles.*` | U (profile_persistence, profile_sections custom-models) | rU | **User** | Machine-level LLM provider definitions (baseUrl, apiKey, per-model caps). Never project. |
| `tui.lastUsedModel` | U (last_used_model_persistence) | rU | **User** | Which LLM model was last selected. Cross-project user preference. |
| `tui.mux` | U (mux_config_persistence) | rU+rP (merged load) | **User** | TUI grid layout. Per-user, not per-repo. (Project override tolerated by the merge, but no writer targets project scope.) |
| `tui.defaultThinkingLevel` | U (default_thinking_level_persistence) | rU+rP (merged load) | **User** | Default thinking level. Per-user. |
| `rlcd.url` | U (tools/src/rlcd/config.rs) | rU | **User** | RLCD decision-engine endpoint. |
| `tools.test.command` | **P** (configure_tools.rs) | rP (bootstrap, reminders, init fallback, checkpoint) | **Project** | Per-repo test command. `configure-tools` writes project scope only. |
| `tools.qualityCheck.commands` | **P** (configure_tools.rs) | rP (bootstrap, reminders, init fallback) | **Project** | Per-repo quality commands. |
| `agent` | **P** (init.rs `write_agent_config`) | rP (review.rs `get_agent_config`, remove_init_files, foundation/guidance) | **Project** | Which agent *harness* was `fspec init`ed (claude/cursor/aider/…). Controls reminder formatting. |
| `research.*` | P (manual, via `fspec research` guidance) | rP then rU (reminders.rs `load_config`) | **Both** (project-first) | Research-tool credentials (perplexity/jira/…). The only key with an explicit user-fallback in a CLI reader. |

### Critical observations

1. **`tools` and `agent` have NO user-scope writer.** The only user-scope writers are the
   `tui.*`, `providers`, and `rlcd` families. So `tools`/`agent` appearing in
   `~/.fspec/fspec-config.json` is, by design, **always contamination** (see BUG-193).
2. **`tools` and `agent` have NO user-scope reader either.** No Rust code reads `tools` or
   `agent` from `~/.fspec/fspec-config.json`. The only "user fallback" for `tools` is in
   `init.rs::apply_tool_command_replacements`, which deep-merges user+project for the doc-template
   placeholder substitution — a TS-parity carryover, not a user-facing feature. So even a
   *deliberate* user-level `tools` is effectively dead except for that init fallback.
3. **`research` is the only key with a genuine two-scope design** (project credentials override
   user credentials). It is the model for what a legitimate cross-scope key looks like.

## 3. The design rule (proposed)

> **User scope holds user-owned state** (`providers`, `tui.*`, `rlcd`, optionally
> `research` credentials). **Project scope holds repo-owned state** (`tools`, `agent`,
> optionally `research` credentials). A key with a single writer owns exactly one scope; the
> other scope must never contain it.

Consequences:
- **`tools` is project-only.** It should not be configurable at user level. (Answers the
  reporter's request #1: "tools should be per project, not system wide.")
- **`agent` is project-only.** It records which harness a *repo* was set up for; it is not a
  user preference. (Answers the reporter's request #2: the `agent` in the global file is not a
  user setting — it leaked there.)
- **`tui.lastUsedModel` is user-only** — unrelated to `agent`; the two coexisting in one file was
  purely a contamination artifact, not a design intent.

## 4. Proposed validation surface

Add a check (candidate: `fspec check` / a new `fspec validate-config` subcommand, or a
`validate-config` core command in `codelet-fspec-core`) that, **for the user-scope file only**:

1. Parses `~/.fspec/fspec-config.json`.
2. Flags any top-level key in the **project-only set** `{ tools, agent }` with a
   `warn` (not `error`) + remediation:
   - `tools` → "Remove `tools` from ~/.fspec/fspec-config.json; configure it per-project with
     `fspec configure-tools` (writes spec/fspec-config.json)."
   - `agent` → "Remove `agent` from ~/.fspec/fspec-config.json; it is set per-project by
     `fspec init`."
3. Optionally lists the user-scope keys it expects (`providers`, `tui`, `rlcd`, `research`) for
   the happy path.

Why `warn` not `error`: `research` legitimately can be user-scoped, and a stray key is a
hygiene issue, not a correctness break (nothing reads it from user scope). A warning keeps the
gate non-blocking while surfacing the leak.

### Implementation sketch (for the developer)

- New module `rust/fspec-core/src/commands/validate_config.rs` exposing the standard
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
- Resolve the user dir via `FSPEC_USER_DIR` / `HOME` (reuse the same convention as
  `profile_sections::fspec_user_dir` — note it currently returns `Option<PathBuf>`; the CLI path
  uses `std::env::var("HOME")`).
- Read-only: never write. Two front doors (LLM dispatcher + clap) converge on this single `run`.
- Register the subcommand in `rust/fspec/src/main.rs` + help config in
  `rust/fspec-core/src/help/configs/`.
- Wire into `fspec check` if a check-aggregator exists (check `commands/check.rs`).

## 5. Test plan (ACDD, integration-first)

Feature file `spec/features/config-scope-validation.feature` (tag `@config-management`,
`@validation`):
- Scenario: user config with a `tools` key → `validate-config` reports a `tools` warning +
  remediation text.
- Scenario: user config with an `agent` key → warning present.
- Scenario: user config with only `providers`+`tui`+`rlcd` → clean, no warnings.
- Scenario: user config with `research` → NOT flagged (legitimate user-scope key).
- Scenario: missing user config file → clean (nothing to validate).
- Scenario: malformed user JSON → reported as a parse warning, not a panic.

Tests live in `rust/fspec-core/tests/validate_config.rs` using `tempfile` + `FSPEC_USER_DIR`
(sandbox HOME per the existing `init.rs` test pattern at `init.rs:363` — redirect HOME so a
developer's real `~/.fspec` never bleeds into the test).

## 6. Open decisions (need product-owner input before specifying)

1. **Drop the user-level `tools` fallback in `init.rs::apply_tool_command_replacements`?**
   Right now `fspec init`'s doc-template substitution falls back to user-scope `tools` if the
   project has none. Under the "tools is project-only" rule this fallback should be removed. It's
   a TS-parity behavior change — confirm before changing. (BUG-193 does NOT touch this; it's a
   separate, smaller decision.)
2. **Warning vs error, and `check` vs standalone subcommand?** Default proposal: `warn` + a
   dedicated `fspec validate-config` that `fspec check` also invokes. Confirm the UX surface.
3. **Should the project-scope file get symmetric validation** (e.g. warn on user-only keys like
   `providers`/`rlcd` being written into a repo's `spec/fspec-config.json`)? Nice-to-have; the
   reported problem is user-scope pollution, so default to user-scope-only validation first.

## 7. Out of scope (deferred)

- Hardening `write_config_with_dirs(ConfigScope::User, …)` to be type-safe against merged input
  (the BUG-193 §6.2 follow-up).
- Migrating/cleaning already-contaminated user configs at startup (a `fspec doctor`-style repair).
  The one-off manual cleanup for the reporter is done; an automated repair is a separate feature.
