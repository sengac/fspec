# AST / Port Research — `report-bug-to-github` (RPC-285)

Worker WB. Source of truth: `src/commands/report-bug-to-github.ts` (415 lines).
Reference port pattern: `list_prefixes` + git-subprocess pattern in
`update_work_unit_status.rs`, version-pin pattern in `init.rs`.

## 1. TS surface (Commander registration, lines 359–413)

```
program.command('report-bug-to-github')
  .description('Report bugs to GitHub with AI-assisted context gathering (system info, git status, work unit, error logs)')
  .option('--project-root <path>', 'Project root directory (auto-detected)')
  .option('--bug-description <text>', 'Brief description of the bug')
  .option('--expected-behavior <text>', 'What you expected to happen')
  .option('--actual-behavior <text>', 'What actually happened')
  .option('--interactive', 'Enable interactive mode with prompts')
```

The action handler prints `\nGathering system context...\n`, calls
`reportBugToGitHub({...})`, then prints either `\n✗ Bug report cancelled\n`
or (on `browserOpened`) `chalk.green('\n✓ Browser opened with pre-filled issue\n')`
followed by `chalk.dim('Review and submit the issue in your browser.\n')`.
On thrown error: `output.error('Error:', msg)` + `process.exit(1)`.

NOTE: the action handler does **not** pass any of the `prompt/confirm/editTitle/
editBody/openBrowser` callbacks. Those are programmatic-only (test) injection
points. Therefore even `--interactive` from the shell never actually prompts —
it only sets `previewShown=true` and otherwise runs the default path.

## 2. Pure / deterministic functions (poll_sync_future-SAFE)

- `formatBugReportMarkdown(context, description, expected, actual, steps)`
  → pure string assembly. Sections: Description, Expected Behavior, Actual
  Behavior, Steps to Reproduce (1-indexed), Environment (fspec version, Node
  version, OS, optional Git branch), Additional Context (work unit block,
  uncommitted-changes note, recent error log fenced block).
- `constructGitHubURL(title, body)`
  → `https://github.com/sengac/fspec/issues/new?title=<enc>&body=<enc>&labels=<enc>`
  with `labels = 'bug,needs-triage'`, encoded via `encodeURIComponent`.
  ⚠️ NO url/percent-encoding crate in either Cargo.toml — must hand-roll an
  `encodeURIComponent`-equivalent (unreserved set = `A-Za-z0-9 - _ . ! ~ * ' ( )`).

## 3. `gatherContext(projectRoot)` — blocking-portable with care

| TS source | Rust port strategy | Sync-safe? |
|-----------|--------------------|------------|
| `package.json` version | pin to const `"0.9.3"` (parity w/ `init.rs` `FSPEC_VERSION`) | ✅ |
| `process.version` (Node) | no Node runtime — emit a fixed/`unknown` token (parity decision needed) | ✅ |
| `process.platform` | `std::env::consts::OS` | ✅ |
| `getCurrentBranch` / `getGitStatus` | **blocking** `std::process::Command::new("git").output()` — exact pattern already used in `update_work_unit_status.rs:479` (`git()` helper, documented poll_sync_future-safe) | ✅ |
| `ensureWorkUnitsFile` → most-recent non-done WU | `io::ensure::ensure_work_units_file(cwd)` (blocking) OR `read_work_units_or_empty` to avoid file creation | ✅ |
| scan `spec/features/*.feature` for `@<workUnitId>` | blocking `std::fs::read_dir` + `read_to_string` | ✅ |
| `.fspec/error-logs/error-latest.json` | blocking `std::fs::read_to_string` + serde | ✅ |

All context-gathering is blocking std → resolves on first poll.

## 4. ⚠️ SCOPE FLAG (the research-EXECUTE analogue) — supervisor decision needed

`openInBrowser({url})` (src/utils/openBrowser.ts) launches the system browser
via the `open` npm package. Characteristics:
- **No-ops** when `NODE_ENV=test` / `VITEST` is set (tests never spawn a browser).
- `wait=false` ⇒ fire-and-forget detached spawn; would technically be
  reproducible with blocking `std::process::Command::spawn` (resolves immediately).
- BUT launching a GUI browser from the LLM **dispatcher** front-door or a CI /
  headless context is an undesirable real side effect.

**RECOMMENDATION (mirror research LIST-vs-EXECUTE split):**
- IN SCOPE for this port: context gathering + markdown formatting + GitHub URL
  construction. The deterministic envelope returns `{title, markdown, context,
  url, browserOpened:false, cancelled:false, previewShown}`. The CLI prints the
  URL for the user to open manually (and/or attempts a best-effort detached
  launch on the standalone-binary path only).
- DEFERRED pending supervisor scope decision: automatic browser launch from the
  dispatcher path, and real interactive stdin prompts (`prompt/confirm/editTitle/
  editBody`) which are NOT poll_sync_future-safe (real stdin blocking) and are
  never wired by the Commander action handler anyway.

NO network IO occurs in this command (GitHub is reached only by the user's
browser); the only external process is `git` (blocking-safe) and the optional
browser launch (flagged above).

## 5. Shared-file wiring owed by SUPERVISOR (Phase C)

1. `canonical.rs` PORTED_COMMANDS += `report-bug-to-github`.
2. `dispatch.rs` move from `run_stub` → `run_ported`, calling
   `report_bug_to_github::run(args_json, project_root)` (signature changes from
   current `run(_args_json)` 1-arg stub to canonical 2-arg).
3. `main.rs` `Mode::ReportBugToGithub` clap variant {project_root, bug_description,
   expected_behavior, actual_behavior, interactive} + `forward!` arm +
   `intercept_ts_help` arm + `mod report_bug_to_github`.
4. `help/configs/mod.rs` register `report_bug_to_github::CONFIG`.

Worker WB owns: `commands/report_bug_to_github.rs` (rewrite stub),
`help/configs/report_bug_to_github.rs`, `fspec/src/report_bug_to_github.rs`
bridge, the two test files, and the help fixture.
