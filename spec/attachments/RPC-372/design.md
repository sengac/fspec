# RPC-372 — Axum attachment viewer HTTP server with markdown rendering

**Parent:** RPC-371 · **Depends on:** none · **Blocks:** RPC-373, RPC-374

## Goal

Port the TypeScript `src/server/attachment-server.ts` + `markdown-renderer.ts` +
`viewer-template.ts` to Rust as a local HTTP viewer server, using the
**fspec.pro axum architecture** (see RPC-371 design.md for the convention table).

The server is a **library** (new crate `codelet/attachment-viewer`, or a module
under `codelet/fspec-tui/src/viewer/`). It is started/stopped by the TUI
(RPC-373/374 wire the keys). This card delivers ONLY the server + renderer +
lifecycle handle; no key wiring.

> **Recommendation:** create a dedicated workspace crate
> `codelet/attachment-viewer` mirroring fspec.pro's `relay-core`
> (`lib.rs` factory + `state.rs` + `handlers/` + `config.rs`). This keeps the
> axum/markdown deps out of `fspec-tui`'s graph except as a single path dep, and
> keeps every file < 300 lines.

## Public API (the lifecycle trio — mirrors TS start/stop/getPort)

```rust
// lib.rs
pub struct ViewerConfig { pub cwd: PathBuf }           // config.rs (fspec.pro: RelayConfig)
#[derive(Clone)] pub struct ViewerState { /* Arc<Inner{ cwd }> */ }   // state.rs

pub fn build_router(state: ViewerState) -> Router;     // routes + .with_state
pub fn build_router_with_config(cfg: ViewerConfig) -> Router; // test-injectable factory

pub struct ViewerHandle { pub port: u16, /* shutdown + JoinHandle */ }
pub async fn start_viewer(cwd: PathBuf) -> anyhow::Result<ViewerHandle>;
impl ViewerHandle { pub async fn stop(self); }
```

`start_viewer`:
1. `cfg = ViewerConfig { cwd }`; `app = build_router_with_config(cfg)`.
2. `listener = TcpListener::bind("127.0.0.1:0").await?`; `port = listener.local_addr()?.port()`.
3. `tokio::spawn(axum::serve(listener, app).with_graceful_shutdown(async { rx.await.ok(); }))`.
4. return `ViewerHandle { port, shutdown: tx, task }`.

`stop(self)`: `let _ = self.shutdown.send(()); let _ = self.task.await;`.

## Routes

- `GET /view/{*path}` → `view` handler (axum 0.8 wildcard capture syntax is
  `{*path}`; `Path(path): Path<String>`).
- `GET /health` → `"ok"` (or `Json({"status":"ok"})`) — parity with TS `/health`.
- Layers (top level): `.layer(CorsLayer::permissive())`, `.layer(TraceLayer::new_for_http())`.

## `view` handler behaviour (port of attachment-server.ts `/view/*`)

Signature: `async fn view(State(state): State<ViewerState>, Path(raw): Path<String>) -> Response`.

1. **Decode** the captured path (percent-decode). The captured `{*path}` is the
   substring after `/view/`.
2. **Resolve & validate** (directory-traversal guard, port of `validatePath`):
   - `abs = if Path::new(&decoded).is_absolute() { decoded } else { cwd.join(decoded) }`.
   - Normalize (resolve `.`/`..`). Use a lexical normalization that does NOT
     require the file to exist (canonicalize fails on missing files — match TS
     which uses `path.normalize`). A lexical normalizer (manual component fold)
     is correct here.
   - If the normalized path does NOT start with the normalized `cwd` → **403
     `Forbidden: Invalid file path`**.
3. **Read & respond by extension** (lowercased):
   - `.md` / `.markdown`: read UTF-8 → `render_markdown(content)` → wrap in
     `viewer_template(title = file basename, content)` → respond
     `200 text/html; charset=utf-8`.
   - else: read bytes → respond `200` with content-type from the map below
     (default `application/octet-stream`):
     `.png→image/png, .jpg/.jpeg→image/jpeg, .gif→image/gif, .svg→image/svg+xml,
      .pdf→application/pdf, .txt→text/plain`.
4. **Errors**: file-not-found (`io::ErrorKind::NotFound`) → **404 `File not
   found`**; any other error → **500 `Internal server error`**. Never panic; no
   `unwrap()` in the request path.

## Markdown renderer (port of markdown-renderer.ts)

`pub fn render_markdown(markdown: &str) -> String` using `pulldown-cmark`
(GFM-enabled). Parity points with the TS `marked` config:

- GitHub-Flavored Markdown (tables, strikethrough, task lists, autolinks).
- Fenced code blocks: a block with info string **`mermaid`** must emit
  `<pre class="mermaid">…escaped code…</pre>` (so client-side mermaid.js renders
  it). All other code blocks emit
  `<pre class="code-block" data-language="LANG"><code>…escaped…</code></pre>`.
- HTML-escape code content (`&`, `<`, `>`, `"`, `'`). A small `html_escape`
  helper (port of `src/server/utils/html-escape.ts`).

> pulldown-cmark gives you fenced-code events with the info string — intercept
> code blocks to apply the mermaid/`code-block` wrapping, and let the rest pass
> through its HTML writer.

## Viewer template (port of viewer-template.ts, trimmed)

`pub fn viewer_template(title: &str, content_html: &str) -> String` returns a
full HTML document:

- `<!DOCTYPE html>` … `<title>` = escaped basename.
- Include **mermaid** via CDN with
  `mermaid.initialize({ startOnLoad: true })` so `<pre class="mermaid">` blocks
  render client-side (this is REQUIRED — FOUNDATION.md and design docs use mermaid).
- A `<style>` block with sane readable defaults (max-width, padding, code block
  styling). Theme toggle / font controls / fullscreen modal / panzoom / Prism are
  **out of scope** for this card (see RPC-371 assumptions).
- `<div class="markdown-content">{content_html}</div>`.

## File layout (all < 300 lines)

```
codelet/attachment-viewer/
  Cargo.toml
  src/lib.rs            # ViewerHandle, start_viewer, stop, build_router(_with_config), re-exports
  src/config.rs         # ViewerConfig
  src/state.rs          # ViewerState (Clone newtype over Arc<Inner>)
  src/handlers/mod.rs   # re-exports
  src/handlers/view.rs  # view handler + path validation + content-type map
  src/handlers/health.rs# health handler (or inline closure)
  src/markdown/mod.rs   # render_markdown
  src/markdown/template.rs # viewer_template
  src/markdown/escape.rs   # html_escape
```

## Scenarios (acceptance criteria)

1. **Renders a markdown attachment as HTML** — `GET /view/<path-to>.md` returns
   `200`, `Content-Type: text/html`, body contains the rendered heading and the
   file basename in `<title>`.
2. **Renders mermaid code blocks for client-side rendering** — a `.md` containing
   a ```` ```mermaid ```` block yields `<pre class="mermaid">` in the HTML.
3. **Serves a binary attachment (image) raw with correct content-type** —
   `GET /view/<path>.png` returns `200`, `Content-Type: image/png`, and the raw bytes.
4. **Blocks directory traversal outside cwd** — `GET /view/../../etc/passwd`
   (or an absolute path outside cwd) returns `403`.
5. **Returns 404 for a missing file** — `GET /view/does-not-exist.md` returns `404`.
6. **Health endpoint responds ok** — `GET /health` returns `200` with `ok`/`{status:"ok"}`.
7. **start_viewer binds a random local port and stop shuts it down** —
   `start_viewer(cwd)` returns a handle with a non-zero `port`; a request to
   `http://127.0.0.1:{port}/health` succeeds; after `stop()`, the task ends.

## Testing

- Integration tests use the fspec.pro harness pattern: build via
  `build_router_with_config(ViewerConfig{cwd: tempdir})`, bind `127.0.0.1:0`,
  `tokio::spawn(axum::serve(...))`, hit it with a `reqwest`/`hyper` client.
  (`reqwest` with `blocking`/async is already a workspace dep.)
- Unit-test `render_markdown` (mermaid wrapping, code-block wrapping, escaping)
  and the path-validation function (traversal rejection, cwd-relative resolution)
  directly.
- Every Gherkin step → `// @step …` comment in the Rust test.

## Definition of done

- 7 scenarios, all green; tests written first (red) then implementation.
- `cargo build -p codelet-attachment-viewer` and `cargo clippy` clean.
- No `unwrap()`/`todo!()`/`unimplemented!()` in production paths.
- All files < 300 lines. Coverage links recorded for every scenario.
