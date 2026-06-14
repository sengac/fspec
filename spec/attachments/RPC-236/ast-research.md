# RPC-236 generate-tags-md — AST Research

## TS source surface (src/commands/generate-tags-md.ts)

- `export async function generateTagsMdCommand(options): Promise<GenerateTagsMdResult>`
  - reads `spec/tags.json`; if missing → `{success:false, error:'tags.json not found: spec/tags.json'}`
  - validates tags.json against `tags.schema.json` (Ajv); on failure →
    `{success:false, error:'tags.json has validation errors: <joined>'}`
  - reads + parses tags.json, calls `generateTagsMd(tagsData)` (src/generators/tags-md.ts)
  - mkdir output dir, writes markdown (NO trailing newline) to `spec/TAGS.md` or `--output <path>`
  - success → `{success:true, message:'Generated <outputRelative> from spec/tags.json'}`
- `export async function generateTagsMdCommandCLI(options)` — prints `✓ <message>` exit 0,
  or `Error: <error>` exit 1.
- `export function registerGenerateTagsMdCommand(program)` —
  `.command('generate-tags-md').option('--output <path>', ...)`.

## Generator (src/generators/tags-md.ts)
Renders header comment + `# fspec Feature File Tag Registry` + per-category sections (table of tags),
combinationExamples, usageGuidelines, addingNewTags, queries, statistics, validation, references.
All sections optional/conditional. Joined by `\n`.

## Closest Rust reference
codelet/fspec-core/src/commands/generate_foundation_md.rs (RPC-233) — same generator shape:
exists-check → schema-validate → parse → render → mkdir → write (no trailing newline) →
`Generated <out> from spec/<src>.json` message. Help config + bridge mirror exactly.

## Shared modules needed
- types/tags.rs::TagsData (EXISTS — has categories + extra catch-all). May need richer fields for
  full generator parity (combinationExamples, usageGuidelines, statistics, etc.) — request from supervisor.
- generators/ module: new `tags_md.rs` generator + a tags-schema validator (analogous to foundation_schema.rs).
  Both live under generators/ (shared file generators/mod.rs — request supervisor wiring).
