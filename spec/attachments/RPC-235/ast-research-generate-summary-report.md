# RPC-235 — generate-summary-report — AST / Port Research

## TS source
- `src/commands/generate-summary-report.ts` — `generateSummaryReport(options)` reads `spec/work-units.json`,
  aggregates counts/points/velocity, renders markdown or JSON, writes to an output file.
- `src/commands/generate-summary-report-help.ts` — rich help config (ported to
  `codelet/fspec-core/src/help/configs/generate_summary_report.rs`).
- Canonical registry: `codelet/fspec-core/src/canonical.rs:376` (`ts_file: src/commands/generate-summary-report.ts`).

## Rust impl under test (already landed)
`codelet/fspec-core/src/commands/generate_summary_report.rs`:
- `struct GenerateSummaryReportArgs` (line 33) — `{ format: Option<String>, output: Option<String> }`, camelCase serde.
- `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` (line 39).
- `fn estimate_of(wu: &Value) -> u64` (line 146) — reads `estimate` from the work-unit `extra`/value, defaulting 0.
- `fn generate_markdown_report(report: &Map<String, Value>) -> String` (line 151) — markdown layout.
- `fn resolve_output(project_root: &Path, file: &str) -> PathBuf` (line 191) — default `spec/summary-report.<md|json>`.

## Behaviour verified
- Reads `spec/work-units.json` directly via `std::fs::read_to_string` (NOT ensure — missing file is an error
  wrapped as `Failed to generate summary report: <message>`).
- Aggregates: `totalWorkUnits`, `byStatus` (insertion-order counts, status default `"unknown"`),
  `totalStoryPoints` (sum estimate, missing = 0), `velocity { completedPoints, completedWorkUnits }` over `status == "done"`.
- `format` default markdown; `json` => `serde_json` pretty (2-space); markdown => `generate_markdown_report`.
- Returns message `✓ Report generated: <outputFile>`.

## Two front doors (verified)
- Dispatcher arm `codelet/fspec-core/src/dispatch.rs:499` → `commands::generate_summary_report::run(args_json, project_root).await`.
- CLI bridge `codelet/fspec/src/generate_summary_report.rs` marshals `--format`/`--output`, prints the message
  or `✗ Failed to generate report: <msg>` (exit 1).

## DRY / SOLID
- Reuses shared `serde_json` Value model and FspecCoreError; no duplicate helpers introduced.
- Status counting kept Value-based to preserve unknown fields (parity with TS `Object.entries`).
