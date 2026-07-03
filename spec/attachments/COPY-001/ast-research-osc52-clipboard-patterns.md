# AST Research — COPY-001 OSC 52 clipboard writer

## Goal
Confirm the generic-writer testing pattern to mirror, and the base64 STANDARD engine usage available in the workspace, before implementing `Osc52Clipboard<W: Write + Send>`.

## Pattern to mirror: MouseTrackingToggle generic writer

AstGrep `pub struct MouseTrackingToggle<W: Write + Send> { $$$FIELDS }` in `codelet/fspec-tui/src/mouse/toggle.rs`:

```
codelet/fspec-tui/src/mouse/toggle.rs:44: pub struct MouseTrackingToggle<W: Write + Send = std::io::Stdout> {
```

Confirms: generic `W: Write + Send = std::io::Stdout`, `with_stdout()` production constructor delegating to `std::io::stdout()`, and `new(writer)` test constructor. `Osc52Clipboard` will follow the same shape.

## base64 STANDARD engine usage in the workspace

AstGrep `STANDARD.encode($X)` across `codelet/`:
- `codelet/common/src/image_dimensions.rs:368` (+ many) — `STANDARD.encode(&raw)`
- `codelet/tools/`, `codelet/cli/` tests also use it.

Import form in those files: `use base64::engine::general_purpose::STANDARD;` then `STANDARD.encode(bytes)`. base64 crate `0.22` is used across the workspace (providers, tools, cli, common, napi). `fspec-tui/Cargo.toml` does NOT yet declare base64 — must add `base64 = "0.22"` under `[dependencies]`.

## Conclusion
- New module `codelet/fspec-tui/src/mouse/clipboard.rs`, exported from `mouse/mod.rs`.
- `Osc52Clipboard<W: Write + Send = std::io::Stdout>` with `with_stdout()` + `new(writer)`.
- `copy(&mut self, text: &str) -> std::io::Result<()>` writes `b"\x1b]52;c;"` + `STANDARD.encode(text.as_bytes())` ascii + `b"\x07"`, then flush.
- Unit tests in-module inject `Vec<u8>` and assert exact bytes (ascii `aGk=`, empty, multiline `YQpi`, emoji `8J+YgA==`).
