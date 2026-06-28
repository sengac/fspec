# AST Research — RPC-372 Axum Attachment Viewer Server

Grounding the design in concrete patterns from BOTH the fspec.pro reference
codebase (the HTTP-server architecture we must follow) and the existing
`codelet/` Rust port.

## 1. fspec.pro — router factory + `.with_state` (the convention)

`ast-grep 'pub fn build_router($$$ARGS) -> Router { $$$BODY }'`:
- `server/src/lib.rs:11` — `pub fn build_router() -> Router` → calls
  `build_router_with_config(config)`; merges sub-routers, applies
  `CorsLayer::permissive()` + `TraceLayer::new_for_http()` at top level.

Sub-routers (e.g. `extension/native-host/src/audio/mod.rs:36`) follow:
```rust
Router::new()
    .route("/api/audio/cover", get(handlers::get_cover_art))
    .with_state(state)
```
→ Our analogue: `Router::new().route("/view/{*path}", get(view)).route("/health", get(health)).with_state(state)`.

## 2. fspec.pro — file-serving handler with content-type + traversal guard

Best reference is `extension/native-host` (serves real files over axum):

`ast-grep 'pub async fn $NAME(State($S): State<$T>, $$$REST) -> $RET { $$$BODY }'`:
- `audio/handlers.rs:255` `get_cover_art(State, Query)` — builds a binary
  response:
  ```rust
  Response::builder()
      .status(StatusCode::OK)
      .header(header::CONTENT_TYPE, mime_type)
      .header(header::CONTENT_LENGTH, data.len())
      .body(Body::from(data))
      .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "..."))
  ```
- `workspace/handlers.rs:198` `read_file(State, Path(file_path): Path<String>)`:
  ```rust
  let decoded = urlencoding::decode(&file_path).unwrap_or(file_path.clone().into());
  let full_path = resolve_path(&config.current_directory, &decoded);
  if !is_within_workspace(&config.workspace_root, &full_path) {
      return (StatusCode::FORBIDDEN, Json(json!({"error":"Access denied"}))).into_response();
  }
  match std::fs::read_to_string(&full_path) {
      Ok(content) => ...Ok,
      Err(_) => (StatusCode::NOT_FOUND, ...).into_response(),
  }
  ```

**Takeaways for our `view` handler:**
- Use `Path<String>` extractor for the `{*path}` wildcard.
- Decode with `urlencoding::decode` (already used in fspec.pro) — avoids adding
  a separate percent-encoding dep; OR use `percent-encoding`. Either is fine.
- Traversal guard = resolve under cwd then `canonical/normalized.starts_with(root)`.
  fspec.pro uses `canonicalize()` for existing paths. NOTE: the TS reference uses
  lexical `path.normalize` so it can 403 a traversal even for a missing file.
  Decision: do the lexical containment check on the JOINED path BEFORE reading,
  so traversal → 403 regardless of existence; then read → 404 on NotFound.
- Build binary responses with `Response::builder().header(CONTENT_TYPE, ...)`;
  build HTML responses the same way with `text/html; charset=utf-8`.
- `error_response(status, msg)` helper pattern → mirror as a small helper.

`workspace/handlers.rs:65` `is_within_workspace` shows the containment idiom:
```rust
let root = Path::new(workspace_root).canonicalize()?;
if let Ok(normalized) = full_path.canonicalize() { return normalized.starts_with(&root); }
```

## 3. fspec.pro — state newtype injected via `State<T>`

`relay-core/src/ws.rs:16` `ws_upgrade(State(state): State<RelayState>, ...)` and the
`#[derive(Clone)] RelayState { inner: Arc<RelayStateInner> }` newtype confirm the
Clone-newtype-over-Arc + `State` extractor pattern → our `ViewerState`.

## 4. codelet/ port — existing target surfaces (context for RPC-373/374, not this card)

- `codelet/fspec-tui/src/views/board.rs:104` `BoardView::handle_event` — has
  `f/F`→OpenChangedFilesView and `c/C`→OpenCheckpointsView arms; NO `a`/`d`.
- `codelet/fspec-tui/src/components/mod.rs:108` `enum Action` (OpenChangedFilesView
  @967, OpenCheckpointsView @979) — where OpenFoundation/OpenAttachment* will go.
- `codelet/fspec-tui/src/app/dispatch_changed_files.rs` — the `try_dispatch_*`
  helper pattern to mirror for a new `dispatch_viewer.rs`.
- `codelet/rpc-types/src/lib.rs:37` `WorkUnitInfo { … pub attachments: Vec<String> }`
  — the data the `A` picker reads.
- `codelet/providers/Cargo.toml:56` `open = "5"` — browser launcher already in the
  workspace (promote to a workspace dep for RPC-373/374).

## 5. Workspace dependency gaps (must add)

`grep` of `codelet/Cargo.toml [workspace.dependencies]` confirms NONE of these
exist yet: `axum`, `tower`, `tower-http`, `pulldown-cmark`/`comrak`, `open`
(only under providers), `urlencoding`/`percent-encoding`. All must be added for
RPC-372.

## Conclusion

The fspec.pro `extension/native-host` file handlers + `server` router factory give
a 1:1 template for an axum file/markdown viewer. Reuse: `build_router(_with_config)`
factory, Clone-state-over-Arc + `State` extractor, `Path<String>` wildcard,
`Response::builder()` with explicit content-type, `urlencoding::decode`, and the
`starts_with(root)` containment guard (applied lexically pre-read for 403-on-missing).
