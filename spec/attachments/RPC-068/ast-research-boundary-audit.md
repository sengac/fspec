# RPC-068 — AST Research

**Card:** RPC-068
**Phase:** specifying → testing transition (AST research required by fspec workflow)
**Method:** `ast-grep --lang rust` pattern matching across the workspace plus targeted `rg`/`grep` audits of file shape.

This file records the AST/source-shape evidence that supports the
boundary-audit scenarios in
`spec/features/rpc-068-final-ts-regression-and-boundary-audit.feature`.

---

## 1. Dependency-rule regression tests are wired in every forbidden crate

### `ast-grep` query

```
ast-grep --pattern 'fn no_codelet_napi_import_in_source() { $$$BODY }' \
         --lang rust  codelet/
```

### Results

| Crate | File | Line |
|---|---|---|
| `codelet-core` | `codelet/core/tests/no_napi_dependency.rs` | 31 |
| `codelet-sessions` | `codelet/sessions/tests/no_napi_dependency.rs` | 34 |
| `codelet-rpc-types` | `codelet/rpc-types/tests/no_napi_dependency.rs` | 35 |
| `codelet-fspec` | `codelet/fspec/tests/no_napi_dependency.rs` | 30 |
| `codelet-fspec-tui` | `codelet/fspec-tui/tests/no_napi_dependency.rs` | 30 |

### `ast-grep` query (transitive-graph counterpart)

```
ast-grep --pattern 'fn no_codelet_napi_in_transitive_dependency_graph() { $$$BODY }' \
         --lang rust  codelet/
```

### Results

| Crate | File | Line |
|---|---|---|
| `codelet-core` | `codelet/core/tests/no_napi_dependency.rs` | 22 |
| `codelet-sessions` | `codelet/sessions/tests/no_napi_dependency.rs` | 24 |
| `codelet-rpc-types` | `codelet/rpc-types/tests/no_napi_dependency.rs` | 25 |
| `codelet-fspec` | `codelet/fspec/tests/no_napi_dependency.rs` | 20 |
| `codelet-fspec-tui` | `codelet/fspec-tui/tests/no_napi_dependency.rs` | 20 |

Each crate contributes two `#[test]` functions, giving 10 total dependency-rule
tests — matching the assertion in Scenario "Dependency-rule regression tests pass
across every forbidden crate".

---

## 2. `SessionManager` lives in `codelet-sessions`, not in `codelet-napi`

### `ast-grep` query

```
ast-grep --pattern 'pub struct $NAME { $$$FIELDS }' \
         --lang rust  codelet/sessions/src/session_manager.rs
```

### Results

```
codelet/sessions/src/session_manager.rs:154:1:pub struct SessionManager {
```

There is no matching `pub struct SessionManager` in `codelet/napi/src/`
because `codelet/napi/src/session_manager.rs` was deleted by RPC-040.

### Filesystem audit

```
$ test ! -f codelet/napi/src/session_manager.rs && echo "DELETED"
DELETED

$ test -f codelet/sessions/src/session_manager.rs && echo "EXISTS"
EXISTS
```

---

## 3. `GLOBAL_CHUNK_CALLBACK` static is removed from executable code

### `rg` audit (across the whole tree)

```
rg "static GLOBAL_CHUNK_CALLBACK" codelet/
```

(zero matches in executable Rust)

The string `GLOBAL_CHUNK_CALLBACK` survives only as:

* doc-string comments in `codelet/sessions/src/background_session.rs` and
  `codelet/sessions/src/lib.rs`;
* string-literal-only test assertions in
  `codelet/napi/tests/global_chunk_callback_napi_test.rs` and
  `codelet/sessions/tests/background_session_shape.rs`, both of which
  assert the static is absent from executable code.

### Replacement wiring (broadcast)

```
codelet/sessions/src/session_manager.rs:
  chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>
  pub fn chunks_tx(&self) -> &broadcast::Sender<(SessionId, StreamChunk)>

codelet/sessions/src/background_session.rs:
  chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>

codelet/sessions/src/handle_impl.rs:
  fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)>
```

The `codelet-napi` adapter (`codelet/napi/src/session_bindings.rs`)
subscribes to that broadcast and republishes each chunk through the
existing JS `ThreadsafeFunction<GlobalChunkCallbackArgs>` so the TS
`sessionSetGlobalChunkCallback` API surface is preserved.

---

## 4. NAPI persistence collapsed to a thin adapter

### `ls codelet/napi/src/persistence/`

```
mod.rs           (1.4 kB)
napi_bindings.rs (35.3 kB)
```

### `ls codelet/core/src/persistence/`

```
blob.rs                 (10.4 kB)   # RPC-034
blob_processing.rs      (10.2 kB)   # RPC-034
history.rs              (10.0 kB)   # pre-RPC-030 lift
lazy_init_tests.rs                  (test helpers)
manifest.rs             (38.1 kB)   # RPC-033
message_envelope.rs     (26.4 kB)   # RPC-031
messages.rs             (16.0 kB)   # RPC-032
messages/index.rs                   # RPC-032 sub-module
mod.rs                  ( 3.4 kB)
sessions.rs             ( 1.3 kB)
tests.rs                (95.6 kB)
```

Every pure-Rust persistence type — `MessageEnvelope`, `MessageStore`,
`SessionStore`, `BlobStore`, `BlobProcessor`, and the history layer —
lives in `codelet-core`. `codelet-napi` only `#[napi]`-wraps them.

---

## 5. Forbidden `use codelet_napi` imports

### `rg` audit

```
rg "use codelet_napi" codelet/core codelet/rpc codelet/rpc-types \
                     codelet/rpc-embedded codelet/rpc-server \
                     codelet/fspec/src codelet/fspec-tui codelet/sessions
```

Results: the only matches are inside the dependency-rule regression
tests themselves (`tests/no_napi_dependency.rs`, `architecture_invariants.rs`,
`rpc_006_source_shape.rs`, `rpc_007_source_shape.rs`), where the literal
string is part of the assertion that the substring is absent from `src/`.
No `src/` file contains this import.

---

## 6. Forbidden `codelet-napi` manifest dependencies

### Per-crate `Cargo.toml` audit

```
$ grep -E "^codelet-napi" codelet/{core,rpc,rpc-types,rpc-embedded,rpc-server,fspec,fspec-tui,sessions}/Cargo.toml
(zero hits)
```

The only `codelet-napi` token in those manifests is in a doc comment
line in `codelet/sessions/Cargo.toml` and similar — these are not
dependency declarations.

---

## 7. TS-facing `index.d.ts` regression — function-surface diff

### Method

```
git show ea0ed0a0:codelet/napi/index.d.ts > /tmp/baseline.dts
grep -oE "export declare function [a-zA-Z_]+" /tmp/baseline.dts | sort -u > /tmp/fns-baseline.txt
grep -oE "export declare function [a-zA-Z_]+" codelet/napi/index.d.ts | sort -u > /tmp/fns-current.txt
comm -23 /tmp/fns-baseline.txt /tmp/fns-current.txt
comm -13 /tmp/fns-baseline.txt /tmp/fns-current.txt
```

### Results

* Functions only in baseline (i.e. removed): **zero**.
* Functions only in current (i.e. added): **5** — `countCheckpoints`,
  `getModelInfo`, `getWorkspaceInfo`, `moveWorkUnitUp`, `moveWorkUnitDown`.

The TS-facing function surface is a strict superset of the baseline.

---

## 8. Conclusion

Every invariant stated in the RPC-068 audit attachment is supported by
either an AST-level structural query or a deterministic source-tree
audit (file presence/absence, manifest grep, identifier grep). The
results above are reproducible from the workspace at the commit RPC-068
landed on.
