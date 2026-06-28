# RPC-371 — Board A/D keys open browser viewer for attachments and FOUNDATION.md

## Problem

On the **Rust port** board view (`codelet/fspec-tui`), the board renders two hints that
promise functionality that does not exist:

- The header chord (`src/views/board/keybinding_shortcuts.rs`) advertises
  `… ◆ D FOUNDATION.md ◆ …`.
- The details strip (`src/views/board/details_strip.rs`,
  `build_attachments_line`) renders `Attachments (use the "A" key to view): …`.

But `BoardView::handle_event` (`src/views/board.rs`) has **no `a`/`A` or `d`/`D`
match arm** — both keys fall through to `EventResult::ignored()`. The supporting
machinery that the TypeScript reference uses (a local HTTP viewer server, a
markdown→HTML renderer, a browser launcher, and an attachment-picker dialog) was
**never ported to Rust**.

This epic ports that machinery and wires both keys end-to-end.

## TypeScript reference (the behaviour we are porting)

Anchor file: `src/tui/components/BoardView.tsx`.

1. **Attachment HTTP server** — `src/server/attachment-server.ts`
   - Started in a `useEffect` when BoardView mounts, on a **random port**
     (`port: 0`), bound to the project `cwd`.
   - Exposes `GET /view/*filepath`. For `.md`/`.markdown` it renders an HTML
     page (`renderMarkdown` + `getViewerTemplate`, mermaid-capable); other files
     (png/jpg/gif/svg/pdf/txt) are served raw with a content-type map.
   - Path-traversal guard: resolved path MUST stay under `cwd`, else `403`.
     Missing file → `404`. Other errors → `500`. Also a `GET /health` → `{status:"ok"}`.
   - Port stored in state via `setAttachmentServerPort`; torn down on unmount.

2. **`D` key → open FOUNDATION.md** — `BoardView.tsx:314-325`
   ```ts
   if (input === 'd' || input === 'D') {
     const foundationPath = 'spec/FOUNDATION.md';
     if (attachmentServerPort) {
       const url = `http://localhost:${attachmentServerPort}/view/${foundationPath}`;
       openInBrowser({ url, wait: false }).catch(...);
     }
     return true;
   }
   ```

3. **`A` key → attachment picker** — `BoardView.tsx:327-333, 592-616`
   - `A` only opens a dialog **if the selected work unit has attachments**
     (`hasAttachments()` guard). No attachments → no-op (still consumes the key).
   - `AttachmentDialog` lists the work unit's attachments; on select it builds
     `http://localhost:PORT/view/<encodeURI(attachment)>` (falling back to a
     `file://` URL when the server is down) and calls `openInBrowser`.

4. **Browser launcher** — `src/utils/openBrowser.ts`: thin wrapper over the
   `open` npm package, no-op in test environments.

## HTTP server architecture — follow fspec.pro (axum)

The project standard for HTTP servers is the `~/projects/fspec.pro` axum
architecture. Replicate its conventions:

| fspec.pro convention | Apply here |
|---|---|
| Binary owns process concerns; library owns router/state/handlers/config | Viewer lives in a **library** module/crate; the TUI owns lifecycle |
| Two-function factory: `build_router()` (env) → `build_router_with_config(cfg)` | `build_router(state)` + `build_router_with_config(cfg)` so tests inject config |
| Router composed with `.merge(...)`, state bound via `.with_state(state)` | `Router::new().route("/view/{*path}", get(view)).route("/health", …).with_state(state)` |
| State = `#[derive(Clone)]` newtype over `Arc<Inner>`, injected via `State<T>` extractor | `ViewerState { inner: Arc<ViewerStateInner { cwd }> }` |
| Layers at top level: `CorsLayer::permissive()` + `TraceLayer::new_for_http()` | same |
| Test harness: bind `127.0.0.1:0`, `tokio::spawn(axum::serve(...))`, return base URL | same — used by integration tests |
| `axum::serve(listener, app).await` (axum 0.8 idiom) | same, with `.with_graceful_shutdown(...)` for clean stop |

**Key difference from fspec.pro:** fspec.pro is a standalone `#[tokio::main]`
binary. Our viewer must be embedded in the running TUI and started/stopped on
the board's lifecycle. So instead of a `main.rs`, expose:

```rust
pub struct ViewerHandle { pub port: u16, shutdown: oneshot::Sender<()>, task: JoinHandle<()> }
pub async fn start_viewer(cwd: PathBuf) -> anyhow::Result<ViewerHandle>;
impl ViewerHandle { pub async fn stop(self); }
```

`start_viewer` binds `127.0.0.1:0`, reads back `local_addr().port()`, spawns
`axum::serve(...).with_graceful_shutdown(rx)` on a tokio task, and returns the
port. `stop()` fires the oneshot and joins the task. This mirrors the TS
`startAttachmentServer`/`stopAttachmentServer`/`getServerPort` trio.

## Children & order

1. **RPC-372** — Axum attachment viewer HTTP server + markdown rendering
   (the infrastructure). No dependencies.
2. **RPC-373** — Wire `D` → FOUNDATION.md. Depends on RPC-372.
3. **RPC-374** — Wire `A` → attachment picker + browser. Depends on RPC-372.

## Crates / dependencies to add (workspace)

None of these are in the `codelet` workspace yet — add to `codelet/Cargo.toml`
`[workspace.dependencies]` and the relevant crate manifests:

- `axum = { version = "0.8", features = [] }` (no `ws` needed)
- `tower = "0.5"`, `tower-http = { version = "0.6", features = ["cors", "trace"] }`
- a markdown→HTML crate: **`pulldown-cmark`** (pure-Rust, GFM) or `comrak`
- `open = "5"` (already used by `codelet/providers`; promote to workspace dep)
- `percent-encoding` / `urlencoding` for `/view/<encoded path>` decode
- `mime_guess` (optional) or a hand-rolled content-type map matching the TS one

## Acceptance themes (epic level)

- Pressing **D** on the board opens the default browser to the rendered
  `spec/FOUNDATION.md`.
- Pressing **A** on a card **with** attachments opens a picker; selecting one
  opens it in the browser. Pressing **A** on a card **without** attachments is a
  silent no-op (key consumed).
- The viewer server renders markdown (incl. mermaid client-side) and serves
  images/pdf raw, with directory-traversal protection scoped to cwd.
- Architecture matches fspec.pro axum conventions; all files < 300 lines; clippy
  clean; tests-first ACDD with full scenario coverage.

## Out of scope (assumptions)

- The elaborate TS viewer chrome (theme toggle, font-size controls, fullscreen
  mermaid modal, panzoom, Prism syntax highlighting) is NOT required for parity
  of the core ask. Mermaid client-side rendering via CDN IS in scope (FOUNDATION
  and design docs contain mermaid). Advanced chrome can be a follow-up.
- Live-reload / websockets are out of scope (fspec.pro's WS machinery is not needed).
