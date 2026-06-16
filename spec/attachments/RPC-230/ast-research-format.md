# AST Research — format (RPC-230)

## TS source
- `src/commands/format.ts` (135 LOC)
- `src/commands/format-help.ts` — rich CommandHelpConfig (name: 'format')
- `src/utils/gherkin-formatter.ts` (421 LOC) — the `GherkinFormatter` class doing the actual work

## Behaviour (`formatFeatures`)
1. file arg → format only that file; `access()` first → ENOENT throws `File not found: <file>`, EACCES throws permission message.
2. no file arg → glob `spec/features/**/*.feature`; empty → `{formattedCount:0}` (no error).
3. Per file: read → `Gherkin.Parser.parse` → `formatGherkinDocument(ast)` → write back. formattedCount++.
4. all-files mode: a parse/FS error on a single file logs a warning (`output.error`) and `continue`s (does NOT abort). Single-file mode lets the outer throw propagate.
5. Returns `{formattedCount}`.

## CLI command (`formatCommand` + `registerFormatCommand`)
- positional `[file]`
- formattedCount===0 → `No feature files found to format`, exit 0
- file arg → `✓ Formatted <file>`
- else → green `✓ Formatted N feature files`
- catch → `output.error('Error:', error.message)`, exit 1

## Formatter layout contract (`GherkinFormatter.format`)
Verified output (cat -A) of a representative file:
```
@tag1$
Feature: Sample$
  As a user$          ← description, 2-space indent
  I want things$
$
  Background: $        ← NOTE trailing space (keyword + ': ' + empty name)
    Given setup$
$
  @s1$
  Scenario: First$
    Given a step$
    When another$
    Then result$
      | a | bb |$       ← table aligned, 6-space indent (step+1)
      | 1 | 22 |$
$
  Scenario Outline: Out$
    Given <x>$
$                       ← blank line before Examples
    Examples:$
      | x |$
      | 1 |$
```
Rules (from gherkin-formatter.ts):
- file ends with exactly one `\n` (`lines.join('\n') + '\n'`)
- indent unit = 2 spaces; scenario keyword at level 1 (2 sp), steps at level 2 (4 sp), tables/docstrings at step+1
- blank line before each feature child; blank line before each Examples block
- tables: per-column max width, `| cell.padEnd(w) | ...|`
- doc strings: delimiter preserved (`"""` or ```` ``` ````), content lines re-indented
- tags each on own line at the element's indent
- comments: re-inserted by line number (`buildCommentMap` + `insertCommentsBeforeLine`)
- description: trimmed per line, max 2 consecutive blank lines, re-indented

## ⚠️ PORT RISK — formatter is NOT yet ported
There is NO `format_gherkin_document` / GherkinFormatter equivalent in codelet/fspec-core. The Rust `gherkin-0.16` crate (`crate::io::gherkin`) is a PARSER only and may NOT expose:
  - comments (the crate likely discards them)
  - exact keyword strings / trailing-space `Background: ` quirk
  - description as raw multi-line text
  - column/table fidelity identical to @cucumber/messages
**This is the crux of the port.** The Rust formatter must be hand-written to reproduce gherkin-formatter.ts byte-for-byte against the gherkin-0.16 AST. Where the crate AST lacks data (e.g. comments, doc-string media types), TESTING phase must determine exact divergence and the supervisor may need to accept a documented "Framing"-style divergence OR the port reuses a line-level approach.
**RECOMMEND**: build the formatter as a new module `codelet/fspec-core/src/io/gherkin_format.rs` (extends the existing io::gherkin module family) — flag to supervisor that this is a NEW shared io module (allowed under playbook §9 "extend existing modules"), owned by this worker but living under io/. Supervisor to confirm whether a new io submodule needs registration in io/mod.rs (shared file — SUPERVISOR ACTION).

## Help — RICH config (NOT bare commander)
`format --help` uses the rich formatter (42-line fixture captured). → normal `help/configs/format.rs` module + register in configs/mod.rs.
Captured fixture: `codelet/fspec/tests/fixtures/help/format.txt`.

## SHARED-FILE CHANGE REQUESTS (supervisor)
1. canonical.rs: add `format` to PORTED_COMMANDS.
2. dispatch.rs: add run_ported arm, remove run_stub arm.
3. main.rs: add `Mode::Format { file: Option<String> }`, forward! arm, `mod format;`, intercept arm calling configs::format::CONFIG.
4. help/configs/mod.rs: add `pub mod format;`.
5. io/mod.rs: register `pub mod gherkin_format;` if a new io submodule is created (SHARED — confirm).
