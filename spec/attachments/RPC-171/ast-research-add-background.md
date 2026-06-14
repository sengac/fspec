# RPC-171 — `add-background` AST / behaviour research

## TS source
`src/commands/add-background.ts` + `src/commands/add-background-help.ts`

## Signature (Commander.js)
```
fspec add-background <feature> <text>
```
- `<feature>`: feature file name (basename, e.g. "login") OR explicit path ending in `.feature`
- `<text>`: user story text, multi-line via literal `\n` sequences passed through the shell

## Behaviour (line-based mutation, NOT AST round-trip)

1. **Empty text guard** — `if (!text || text.trim().length === 0)` →
   `{ success:false, error:'Background text cannot be empty' }`.
2. **Feature path resolution**:
   - If `feature.endsWith('.feature')`: `featurePath = join(cwd, feature)`; `access()` →
     ENOENT → `Feature file not found: <feature>`.
   - Else: `glob(['spec/features/**/*.feature'], {cwd, absolute:false})`, match by
     basename (`f.split('/').pop().replace('.feature','') === feature`). No match →
     `Feature file not found: <feature>`.
3. **Read** file as utf-8.
4. **Pre-parse validate** with `@cucumber/gherkin`. On throw →
   `Invalid Gherkin syntax in feature file: <msg>`.
5. **Split lines** `content.split('\n')`.
6. **Find Feature line**: first line whose trim starts with `Feature:`. None →
   `No Feature line found in file`.
7. **Find doc-string end** after the Feature line: walk forward, track `"""` open/close.
   Stop early at a line starting with `Background:`/`Scenario:`/`@` (when not inside docstring).
   `docStringEndIndex` defaults to `featureLineIndex` (no docstring).
8. **Detect existing Background** from `docStringEndIndex+1`: stop at `Scenario:`/`@`
   before Background found; a `Background:` line sets `existingBackgroundStart`; Background
   ends at next `Scenario:`/`@`/`Feature:` (backing off trailing blank lines), or EOF.
9. **Build new Background block**:
   ```
   ['  Background: User Story', '    <line1>', '    <line2>', ...]
   ```
   Title is ALWAYS `User Story`; each text line indented 4 spaces.
10. **Insert/replace**:
    - existing Background → `lines.splice(start, end-start+1, ...bgLines, '')`
    - else → `lines.splice(docStringEndIndex+1, 0, '', ...bgLines, '')`
11. **Re-parse validate** the joined content. On throw → `Generated invalid Gherkin: <msg>`.
12. **Write** file (`join('\n')`).
13. Success → `{ success:true, message:'Added background to <feature>' }`.

## CLI wrapper
`addBackgroundCommand`: on `!success` → `output.error('Error:', error)` + exit 1;
else `output.log('✓', message)` + exit 0.

## Rust port plan
- Reuse `crate::io::gherkin::parse_feature_lenient` for pre/post validation.
- Line-based `split('\n')` / `Vec<String>` / `splice`-equivalent (`Vec::splice`) / `join("\n")`.
- Path resolution: `feature.ends_with(".feature")` → direct; else walk `spec/features/**/*.feature`.
  Can reuse glob via `walkdir`/existing helper if available, else `std::fs` recursive walk.
- Args shape: `{ feature: String, text: String }` (camelCase n/a — both single words).
- Errors → `FspecCoreError::InvalidArgs { command:"add-background", reason }`; IO via `Io`.
- Response JSON envelope: `{ success:true, message }`.

## Shared-file needs
- Path glob helper for `spec/features/**/*.feature` basename match — check if a shared
  helper exists; otherwise implement locally with `std::fs` recursion (no new shared file).
