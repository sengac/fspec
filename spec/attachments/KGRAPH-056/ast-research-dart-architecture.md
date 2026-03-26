# AST Research: AstGrep and AstGrepRefactor Tool Architecture

## Files Requiring Modification for Dart Support

### 1. Rust Core Tools (rig agent tools)

| File | Change Required |
|------|----------------|
| `codelet/tools/Cargo.toml` | Add `tree-sitter-dart = "0.1.0"` dependency |
| `codelet/tools/src/dart_lang.rs` | **NEW** — DartLang struct implementing Language + LanguageExt traits |
| `codelet/tools/src/lib.rs` | Add `pub mod dart_lang;` |
| `codelet/tools/src/astgrep.rs` | Modify `parse_language()` to handle "dart" → DartLang; add Dart to `get_extensions()`; add Solidity/Nix/Hcl; update error messages |
| `codelet/tools/src/astgrep_refactor.rs` | Same modifications as astgrep.rs — parallel `parse_language()`, `supported_languages()` |

### 2. NAPI Bindings (standalone reimplementation for TypeScript access)

| File | Change Required |
|------|----------------|
| `codelet/napi/src/astgrep.rs` | Has its own `parse_language()` — needs same Dart handling |
| `codelet/napi/Cargo.toml` | Add `tree-sitter-dart = "0.1.0"` or use workspace dep |

### 3. TypeScript Tool Description

| File | Change Required |
|------|----------------|
| `src/research-tools/ast.ts` | Add "dart" to language lists in help config and --lang option |

### Architecture Notes

- Both tools use `parse_language()` which calls `SupportLang::from_str()` — Dart must be intercepted BEFORE this call
- Both tools' rig::tool::Tool descriptions auto-generate JSON schemas via `schemars` from Args structs — the language field is a `String`, not an enum, so no schema change needed
- The NAPI layer has its OWN parallel implementation that needs the same changes
- All 5 provider registration sites (claude, openai, gemini, zai, codex) just chain `.tool(AstGrepTool::new(session_id))` — no changes needed there
- DeepSearch only uses AstGrepTool (read-only) — no changes needed

### Key Pattern: LanguageChoice Enum

Create a shared enum to unify SupportLang and DartLang:

```rust
pub enum LanguageChoice {
    Standard(SupportLang),
    Dart(DartLang),
}
```

Both `AstGrepTool::parse_language()` and `AstGrepRefactorTool::parse_language()` return this enum.
The NAPI `parse_language()` in `codelet/napi/src/astgrep.rs` also needs updating.
