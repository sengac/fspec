# Research: Read tool PDF visual mode — unbounded image count, ignored `limit`, no vision awareness

**Work unit:** BUG-168
**Date:** 2026-09-02
**Source report:** `FIXTHIS.txt` (repo root)

## 1. Symptom (from FIXTHIS.txt)

The agent called:

```
Read({ file_path: '.../Academic Progression – Requirements [Maintained].pdf', limit: 4, offset: 1 })
```

and the tool returned **8+ `[Image]` parts** (67-page document) instead of 4 pages. The
transcript shows the assistant saying "reading the PDF, pages 1-4" and then receiving a
wall of images with no way to stop it.

Two explicit requirements from the report:

1. **Limit the number of images** the Read tool can return for a PDF, and **make it configurable**.
2. **Take the provider model's vision capability into account** ("as well as if it's an
   image model in the provider config").

## 2. Root cause

### 2.1 `offset`/`limit` are silently ignored for PDFs (primary bug)

`rust/tools/src/read.rs`:

- `ReadArgs` (line ~222) declares `offset: Option<usize>` and `limit: Option<usize>`.
- The LLM-facing `definition()` (line ~249) advertises offset/limit for all files and only
  says PDFs are "exempt from the token limit" — it never says offset/limit are ignored.
- The PDF branch (lines 330–388) calls:
  - text mode → `read_pdf_from_bytes(&binary_content, &file_path_str)` (line 361)
  - images mode → `extract_pdf_images(...)` (line 370)
  - visual mode → `render_pdf_pages(...)` (line 380)

  **None of these receives `args.offset` or `args.limit`.** `args.offset`/`args.limit` are
  only consumed by the text/SVG/ipynb branches (lines 321, 397–400).

`rust/tools/src/pdf.rs` confirms all three functions process the whole document:

- `read_pdf_from_bytes` (line 93): loops `for page_num in 1..=page_count` → all pages.
- `render_pdf_pages` (line 179): loops `for (index, page) in pdf_pages.iter()` → **all
  pages rendered to 150-DPI PNG and base64-encoded** (`RENDER_SCALE = 150.0/72.0`, line 176).
- `extract_pdf_images` (line 255): iterates `doc.objects` → **all embedded images**.

So `limit: 4` is accepted by the tool schema and then dropped. The LLM is misled into
believing it paginated the file.

### 2.2 Second conversion layer has no cap either

`rust/patches/rig-core/src/agent/prompt_request/streaming.rs` →
`parse_tool_result_content` (lines 119–250):

- For any JSON with a `pages` array + `total_pages` (the visual-mode shape), it converts
  **every** `pages[i]` into a `ToolResultContent::image_base64(...)` part (lines 153–180,
  plus the nested-text double-serialization twin at 206–244).
- Only per-image defenses exist: EXT-016 pixel-dimension check
  (`check_image_dimensions`) and per-page rejection. **No count cap.**
- `MAX_TOOL_RESULT_TEXT_BYTES` (64 KiB, line 36, `bound_tool_result_text`) only bounds the
  *text* fallback path — image parts bypass it entirely.

### 2.3 No configurable limit exists anywhere

Grepped the whole workspace for `CODELET_*_PAGES`, `MAX_PDF`, `max_images`,
`max_pdf_pages` → **no matches**. Existing related knobs (none of which cover this):

| Knob | Location | Scope |
|---|---|---|
| `CODELET_MAX_FILE_TOKENS` (default 25 000) | `rust/common/src/token_estimator.rs:88` | Text files only; PDFs explicitly exempt via `ExemptFileType::Pdf` (`rust/tools/src/file_type.rs:20`) |
| `MAX_TOOL_RESULT_TEXT_BYTES` = 64 KiB | rig patch `streaming.rs:36` | Text tool results only |
| `MAX_IMAGE_BASE64_BYTES` = 5 MB | `rust/tools/src/read.rs:43` | Single-image files only |
| `MAX_IMAGE_PIXEL_DIMENSION` = 5999 px | `rust/common/src/image_dimensions.rs:20` | Per-image dimension check |

### 2.4 Vision capability is tracked but never consumed by tools

Provider/model config already carries the flag:

- Custom providers: `ModelDef.supports_vision` — `rust/providers/src/custom/config.rs:187`
  (per-model, `#[serde(default)]` = false).
- Cloud/built-in: `supports_vision: model.has_capability(Capability::Vision)` —
  `rust/sessions/src/cloud_models.rs:112,265` (models.dev registry).
- Wire type: `codelet_rpc_types::ModelInfo.supports_vision`
  (`rust/rpc-types/src/lib.rs:363`), populated in `resolve_model_info`
  (`cloud_models.rs:252`).

But the flag is used **only cosmetically**:

- TUI header `[V]` badge — `rust/fspec-tui/src/views/agent/header_build.rs:68`
- Model-selector `[V]` badge — `rust/fspec-tui/src/views/model_selector/rows.rs:125`

Nothing in the tool layer, agent loop, or rig patch consults it. Consequences:

- A non-vision model (e.g. deepseek) still receives full-page PNGs; its provider
  implementation replaces them with `[Image]` text placeholders
  (`patches/rig-core/src/providers/deepseek.rs:225`, mistral drops them entirely) — i.e.
  the context was burned for nothing.
- The Read tool's default mode is `visual` (line 333 of read.rs) regardless of whether
  the active model can see at all.

### 2.5 Why tools can't see the model today (plumbing gap)

`ReadTool` holds only `session_id: Uuid` (`read.rs:111–122`). There is no channel from
session model state to tools. Existing patterns for exactly this kind of session-scoped
plumbing (both already used by tools):

1. **Global callback + `OnceLock`** in `rust/tools/src/facade/wrapper.rs` — e.g.
   `GET_EFFECTIVE_CWD_CALLBACK` (line 637, GIT-020 isolation context), registered at
   session init, keyed lookup by session id (line 734–744).
2. **`SessionRegistry<T>`** — `rust/tools/src/session_registry.rs`, a
   `RwLock<HashMap<Uuid, T>>` used by `tool_progress.rs`, `bash.rs`, etc.

Session-side state to read from: `BackgroundSession` keeps `provider_id` / `model_id`
(AtomicString-like `write().expect(...)` fields, `rust/sessions/src/background_session.rs:870`)
and cached limits via `set_model_limits` (line 877). Model switching happens in
`SessionManagerHandle::set_model` (`rust/sessions/src/handle_impl.rs:1258`), which calls
`apply_model_selection` (`rust/sessions/src/model_resolution.rs:28`) — that function
already resolves per-model values from the `ProviderManager`; a `supports_vision`
resolution would slot in alongside `context_window`/`max_output_tokens`.

Note: `codelet-sessions` depends on `codelet-tools` (sessions/Cargo.toml:20) while
`codelet-providers` also depends on `codelet-tools` (providers/Cargo.toml:11) — so a
registry in `codelet-tools` populated from the sessions layer is dependency-safe.

## 3. Impact / blast radius

- A 67-page PDF at 150 DPI ≈ 200 KB–1 MB+ of base64 **per page** → 10–60 MB of base64 in
  one tool result. Context-window blowout, slow/failed compaction, and provider
  request-size failures.
- `IMAGE_TOKEN_ESTIMATE` is a flat 85 tokens per image
  (`rust/core/src/message_estimator.rs:60`) — the compaction estimator drastically
  undercounts real PDF-page cost (a 150-DPI letter page is typically 1–2k tokens of
  image data on Claude), so compaction thresholds don't trigger either.
- `limit`/`offset` are advertised in the tool description, so every LLM conversation
  about PDFs can be fooled the same way.
- **Existing tests pin the buggy behavior:** `test_visual_mode_includes_page_count`
  (`rust/tools/tests/pdf_read_test.rs:209`) asserts *all 25* pages are rendered, and the
  feature scenario "Visual mode includes page count for context awareness"
  (`spec/features/add-pdf-reading-support-to-read-tool.feature:81–85`) says "all 25 pages
  should be rendered". Both must be amended as part of the fix (ACDD: spec → test → code).

## 4. Fix surface (files)

| File | Change |
|---|---|
| `spec/features/add-pdf-reading-support-to-read-tool.feature` | Amend the 25-page scenario (limit semantics + default cap); add scenarios: `limit` honored, truncation notice, configurable default, non-vision model fallback to text |
| `rust/tools/tests/pdf_read_test.rs` | Update 25-page test; new tests for limit/offset, cap, truncation notice, env-var override |
| `rust/tools/src/pdf.rs` | Add `offset`/`limit` params (start page 1-based, max pages) to `read_pdf_from_bytes`, `render_pdf_pages`, `extract_pdf_images`; add `returned_pages`/`truncated` fields to output structs |
| `rust/tools/src/read.rs` | Pass `args.offset`/`args.limit` through all three PDF branches; apply configurable default cap; emit a "pages X–Y of Z, call Read again with offset=Y+1" notice; optional non-vision guard |
| new (e.g. `rust/tools/src/pdf_limits.rs` or extend `limits.rs`) | Configurable default page cap: env var (e.g. `CODELET_MAX_PDF_PAGES`, default ~20, mirroring `CODELET_MAX_FILE_TOKENS` pattern in `common/src/token_estimator.rs`) and/or fspec-config.json key |
| `rust/tools/src/session_registry.rs` usage (new registry entry) | Session-scoped model capabilities (at least `supports_vision: bool`), populated from sessions layer at session create + `set_model` |
| `rust/sessions/src/background_session.rs` / `handle_impl.rs` / `model_resolution.rs` | Populate the registry from `apply_model_selection`'s resolution (add `supports_vision` to `ResolvedModelLimits` or a sibling resolver) |
| `rust/patches/rig-core/src/agent/prompt_request/streaming.rs` | Defense-in-depth: cap the number of `ToolResultContent::Image` parts produced by `parse_tool_result_content` (mirrors the EXT-016 "layers" approach) |
| `rust/patches/rig-core.patch` | Regenerate/keep in sync (patch file checked in at `rust/patches/`) |

## 5. Design notes / open questions (red cards)

1. **Default cap value.** Suggestion: 20 pages (≈ matches the 2000-line text default's
   spirit and keeps a single Read under ~1 MB of context for typical documents). Must be
   decided before scenarios are written.
2. **Non-vision behavior.** Two candidates:
   a. Silent default: `pdf_mode` unset + model without vision → auto-use `text` mode and
      note it in the output. (Friendly, no extra LLM round-trip.)
   b. Hard error with hint: "active model has no vision; use pdf_mode='text'".
   Recommendation: (a), with a one-line note in the output, matching the
   "invalid mode falls back to visual" leniency already in the spec.
3. **`offset`/`limit` semantics in `images` mode.** Natural reading: limit = max embedded
   images returned, offset = skip N. Confirm.
4. **Truncation notice.** Should mirror `format_truncation_warning`
   (`rust/tools/src/truncation.rs`) but per-page: e.g.
   `Rendered 4 of 67 pages (limit). Continue with offset=5.` — the LLM needs the total
   page count to paginate, which `total_pages` already carries.
5. **Where the cap lives: tool vs rig patch.** Tool-side is the right place (source of
   truth, visible to both front doors: LLM dispatcher and CLI). The rig-patch cap is only
   a belt-and-braces guard for *other* tools (MCP etc.) that emit `pages` arrays.
6. **Config surface.** Env var only (cheapest, matches `CODELET_MAX_FILE_TOKENS`) or also
   `spec/fspec-config.json` (`tools` key exists, see `spec/fspec-config.json`)? Decision
   needed; env var is sufficient for the reported bug.

## 6. Verification plan

- `cargo test -p codelet-tools --test pdf_read_test` (updated + new scenarios, red → green).
- New registry plumbing test: register capability for a session, ReadTool consults it
  (follow `pre_tool_hook` / isolation-context test patterns).
- `cargo clippy -p codelet-tools` + `cargo clippy -p codelet-sessions` (workspace denies
  `unwrap_used`/`expect_used` in production code).
- `fspec validate` + `fspec validate-tags` after feature-file edits.
