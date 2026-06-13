# Merman-based Mermaid validation — feasibility & design

Research attachment shared by **RPC-178** (add-diagram), **RPC-233**
(generate-foundation-md), and **RPC-170** (add-attachment). These three cards
were originally shipped with a "Framing A" divergence: the TypeScript commands
call `validateMermaidSyntax` (a real `mermaid.parse()` / `mermaid.render()` run
inside jsdom), but the Rust ports either reduced this to a pure-regex
pre-check (RPC-178, RPC-233) or skipped Mermaid validation entirely (RPC-170).

This document records the investigation that justifies replacing the
Framing-A shortcut with a **real Rust-native Mermaid parser**:
[`merman`](https://github.com/Latias94/merman).

## 1. What the TypeScript baseline actually does

`src/utils/mermaid-validation.ts` → `validateMermaidSyntax(code)`:

1. **Pre-check (pure string/regex), runs first:**
   - `/subgraph\s+"[^"]+"/` → rejects quoted subgraph titles with
     `"Quoted subgraph titles are not supported. Use: subgraph ID[Title]"`.
   - For every `subgraph <ID>` (via `matchAll(/subgraph\s+(\S+?)(?:\s*\[|\s|$)/g)`),
     the ID must match `/^[a-zA-Z0-9_-]+$/`, else
     `"Invalid subgraph identifier '<id>'. Use only letters, numbers, underscores, and hyphens"`.
2. **Full parse:** spins up jsdom, mocks `getBBox`/`getComputedTextLength`/
   `screen`/`parentRule`, then runs `mermaid.render()`. ANY syntax error
   surfaced by Mermaid itself fails validation.

Consumers of `validateMermaidSyntax`:
- `src/commands/add-diagram.ts` — validates user-supplied diagram code (RPC-178).
- `src/generators/foundation-md.ts` + `src/commands/generate-foundation-md.ts`
  — validates generated diagrams before emitting them (RPC-233).
- `src/utils/attachment-mermaid-validation.ts` (used by `add-attachment.ts`)
  — validates `.mmd` / `.mermaid` files and every ` ```mermaid ` fence inside
  `.md` files (RPC-170).

## 2. The library: merman

- crates.io: `merman-core = "0.8.0-alpha.1"` (parser + semantic model, headless),
  published alongside `merman`, `merman-render`, `merman-ascii`, etc.
- Parity target: upstream **mermaid @11.15.0**.
- License: `MIT OR Apache-2.0` (compatible with fspec MIT).
- For validation we only need the **parser**, so we depend on `merman-core`
  directly (parser-only; no `render`/`ascii`/`raster`/jsdom/DOM, no heavy
  SVG/raster deps).

### Toolchain feasibility (verified)

- merman declares `edition = "2024"`, `rust-version = "1.95"`.
- Installed toolchain in this repo: **rustc 1.91.1**.
- `cargo check -p merman-core` **compiles cleanly on 1.91.1** despite the
  advisory MSRV (the `rust-version` field is advisory; no 1.92–1.95-only API is
  actually hit by the parser crate). Verified 2026-06-13.

### API used for validation

```rust
use merman_core::{Engine, ParseOptions};

let engine = Engine::new();
match engine.parse_diagram_sync(code, ParseOptions::strict()) {
    Ok(Some(_)) => /* valid: a diagram was detected and parsed */,
    Ok(None)    => /* no diagram type detected → treat as invalid */,
    Err(e)      => /* invalid: detection or parse error, message in `e` */,
}
```

`Engine` is cheap to construct (owns pinned-baseline registries). The work is
CPU-bound and synchronous (`*_sync`); no executor needed. `ParseOptions::strict()`
returns errors instead of producing an `error` diagram model.

### Observed behaviour (probe run 2026-06-13)

| Input | Result |
|-------|--------|
| `flowchart TD\n A[Start] --> B[Done]` | `Ok(Some(flowchart-v2))` |
| `sequenceDiagram\n Alice->>Bob: Hi` | `Ok(Some(sequence))` |
| `this is not a diagram at all` | `Err` — "No diagram type detected …" |
| `flowchart TD\n A[Start --> B[Done` | `Err` — "Diagram parse error (flowchart-v2): … Unterminated node label (missing `]`)" |
| `` (empty) | `Err` — "No diagram type detected …" |
| `flowchart TD\n subgraph "Quoted"\n A-->B\n end` | **`Ok(Some)`** — merman ACCEPTS quoted subgraph titles |

## 3. Key divergence: quoted subgraphs

merman **accepts** `subgraph "Quoted"`, whereas the TS pre-check **rejects** it
with a canonical message. Real upstream mermaid behaviour aside, the fspec
contract (and existing feature scenarios) expect the rejection + exact message.

**Decision:** keep the existing pure-string pre-checks (quoted subgraph title +
invalid subgraph identifier) for byte-exact error-message parity, AND run the
merman parse afterwards as the comprehensive syntax gate. Pre-check first
(so its canonical messages win), then merman.

This means the new validator is a strict superset of both the old Rust
Framing-A check and (effectively) the TS behaviour: everything the regex
rejected is still rejected with the same message, and additionally any genuine
syntax error merman detects is now rejected too.

## 4. Design

Introduce one shared module: `codelet/fspec-core/src/utils/mermaid_validation.rs`

```
pub fn validate_mermaid_syntax(code: &str) -> Result<(), String>
    1. pre-check: quoted subgraph title        → canonical Err message
    2. pre-check: invalid subgraph identifier  → canonical Err message
    3. merman Engine::parse_diagram_sync(strict):
         Ok(Some) => Ok(())
         Ok(None) => Err("No diagram type detected ...")  (parity wording)
         Err(e)   => Err(e.to_string())

pub fn extract_mermaid_from_markdown(content: &str) -> Vec<String>
    // ```mermaid\n ... \n``` fences, mirrors attachment-mermaid-validation.ts

pub fn should_validate_mermaid(path) / validate_mermaid_attachment(path)
    // .mmd / .mermaid → validate whole file; .md → validate each fence
```

Rewire the three existing call sites onto this module:
- `commands/add_diagram.rs`: replace `validate_mermaid_subgraph` with
  `validate_mermaid_syntax`.
- `generators/foundation_md_util.rs`: `validate_mermaid` delegates to
  `validate_mermaid_syntax` (generators + generate_foundation_md inherit it).
- `commands/add_attachment.rs`: stop skipping; validate `.mmd/.mermaid/.md`
  attachment sources before copy.

## 5. Risks / parity checks to assert in tests

1. **Generated-diagram parity (RPC-233):** every diagram the FOUNDATION.md
   generator emits (incl. event-storm `subgraph ID["⚡ label"]` forms) MUST
   parse `Ok` under merman, otherwise `generate-foundation-md` would start
   flagging them invalid and break the "valid foundations render byte-identical"
   rule. Add explicit tests over the generator's diagram shapes.
2. **Add-attachment behaviour change (RPC-170):** validation was previously a
   no-op. Restoring it can now reject `.mmd/.mermaid/.md` attachments that
   contain invalid Mermaid. This is the intended TS-parity behaviour and is
   captured as new acceptance criteria.
3. **Error-message wording:** pre-check messages are byte-stable; merman's
   parse-error wording is NOT guaranteed stable across merman versions, so
   scenarios assert on substrings / the "Invalid Mermaid syntax:" prefix rather
   than the full merman message.
