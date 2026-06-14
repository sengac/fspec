# RPC-167 — `add-architecture` AST / behaviour research

## TS source
`src/commands/add-architecture.ts` + `src/commands/add-architecture-help.ts`

## Signature (Commander.js)
```
fspec add-architecture <feature> <text>
```
- `<feature>`: feature file name (basename) OR explicit path ending in `.feature`
- `<text>`: architecture documentation text, multi-line via literal `\n`

## Behaviour (line-based mutation, NOT AST round-trip)

1. **Empty text guard** — `if (!text || text.trim().length === 0)` →
   `{ success:false, error:'Architecture text cannot be empty' }`.
2. **Feature path resolution** — IDENTICAL to add-background:
   - `.feature` suffix → `join(cwd, feature)` + `access()` (ENOENT → `Feature file not found: <feature>`).
   - Else glob `spec/features/**/*.feature`, basename match; no match → `Feature file not found: <feature>`.
3. **Read** utf-8.
4. **Pre-parse validate**; throw → `Invalid Gherkin syntax in feature file: <msg>`.
5. **Split lines** `content.split('\n')`.
6. **Find Feature line**; none → `No Feature line found in file`.
7. **Detect existing doc string** after Feature line: walk forward; stop at line starting
   `Background:`/`Scenario:`/`@`. First bare `"""` → `existingDocStringStart`; second bare
   `"""` → `existingDocStringEnd`, break.
8. **Build new doc string block**:
   ```
   ['  """', '  <line1>', '  <line2>', ..., '  """']
   ```
   Each text line indented 2 spaces; the `"""` fences indented 2 spaces.
9. **Insert/replace**:
   - existing docstring → `lines.splice(start, end-start+1, ...docLines)`
   - else → `lines.splice(featureLineIndex+1, 0, ...docLines)`
10. **Re-parse validate**; throw → `Generated invalid Gherkin: <msg>`.
11. **Write** (`join('\n')`).
12. Success → `{ success:true, message:'Added architecture documentation to <feature>' }`.

## CLI wrapper
`addArchitectureCommand`: `!success` → `output.error('Error:', error)` + exit 1;
else `output.log('✓', message)` + exit 0.

## Difference vs add-background
- Inserts a triple-quoted DOC STRING (`"""`) rather than a `Background:` block.
- No User-Story title; no Background-section detection; uses Feature-line+1 insert point.
- Detects existing doc string by paired bare `"""` lines.
- Success message differs: "Added architecture documentation to ...".

## Rust port plan
- Reuse `crate::io::gherkin::parse_feature_lenient` for pre/post validation.
- Line-based `Vec<String>` splice / `join("\n")`.
- Shared path-resolution helper with add-background (basename glob over spec/features).
- Args shape: `{ feature: String, text: String }`.
- Errors → `FspecCoreError::InvalidArgs { command:"add-architecture", reason }`; IO via `Io`.
- Response JSON envelope: `{ success:true, message }`.

## Shared-file needs
- Same path glob helper concern as add-background; implement locally if no shared helper.
