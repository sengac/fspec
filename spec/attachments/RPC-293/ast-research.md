# RPC-293 retag — AST Research

## TS source surface (src/commands/retag.ts)

- `export async function retag(options: {from,to,dryRun,cwd}): Promise<RetagResult>`
  - rejects when `!from || !to` → `{success:false, error:'Both --from and --to are required'}`
  - rejects when `!to.startsWith('@')` OR `!/^@[a-z0-9-#]+$/.test(to)` →
    `{success:false, error:'Invalid tag format: "<to>". Valid format is @lowercase-with-hyphens'}`
  - globs `spec/features/**/*.feature` (relative). No files → `{success:true, fileCount:0, message:'No feature files found'}`
  - per file: count whole-word occurrences of `from` via regex `(^|\s)<escapedFrom>(?=\s|$)` /gm
  - no matching files → `{success:false, error:'Tag <from> not found in any feature files'}`
  - dryRun → `{success:true, fileCount, occurrenceCount, message:'Would rename <from> to <to> in N file(s) (M occurrence(s))', files}`
  - real: replace `(^|\s)from` → `$1to`, then PARSE replaced content with @cucumber/gherkin;
    on parse failure → `{success:false, error:'Validation failed after renaming in <file>: <msg>'}` (aborts, no further writes)
    else write file. Final → `{success:true, fileCount, occurrenceCount, message:'Renamed ... All modified files validated successfully.', files}`
- `export async function retagCommand(options)` — CLI:
  - failure → `Error: <error>` exit 1
  - dryRun → `Dry run mode - no files modified` + cyan summary + `  - <file>` list, exit 0
  - real with files → `✓ <message>` + `Modified files:` + `  - <file>` list, exit 0
  - else `<message>`
- `export function registerRetagCommand(program)` —
  `.command('retag').option('--from <tag>').option('--to <tag>').option('--dry-run')`.

## IMPORTANT DIVERGENCE from worker brief
The brief says retag mutates `spec/tags.json` registry AND feature files. The ACTUAL TS source ONLY
touches `spec/features/**/*.feature` (regex replace + Gherkin validation). It does NOT read or write
tags.json. We port faithfully to TS source: feature files only. FLAGGED IN REPORT.

Note: TS uses flag options `--from`/`--to` (NOT positional args), despite retag-help.ts documenting
positional `<old-tag> <new-tag>`. The Commander.js registration is the runtime truth → flags.
Help-text divergence policy ("Framing A"): help doc is canon only when CLI discards results — here the
CLI DOES use results, so we mirror the actual flag surface. FLAGGED — supervisor decision.

## Shared modules
- io/feature_glob.rs::glob_feature_files (EXISTS — relative paths, alpha sort)
- io/gherkin.rs::parse_feature_lenient (EXISTS — for post-replace validation)
- Whole-word regex replace: implement locally (regex crate) — request confirmation regex crate available.

## Closest Rust reference
codelet/fspec-core/src/commands/delete_features.rs (RPC-218) — glob + per-file mutation + envelope JSON.
