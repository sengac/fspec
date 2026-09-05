# PROV-144 Research: Per-profile "Max Images" limit + Read tool image budget enforcement

**Status:** Research complete — ready for Example Mapping / spec drafting
**Author:** auto-generated (code reading)
**Date:** 2026-09-04

## Goal

Add a new **Max Images** field to the `/provider` view's OpenAI API profile
create/edit form:

- a numeric field with a **default of 4** (most vision-capable models support
  several images per turn),
- **`0` means no vision at all** — the Read tool must **fail** image reads
  (the model cannot see images),
- **`n >= 1`** caps how many images the Read tool may return in a single tool
  result; exceeding the cap **fails the tool call with a message** naming the
  limit and what to do about it.

Two things to support:
1. **Config plumbing** — a new profile field, persisted per profile.
2. **Read tool enforcement** — messaging that limits image reads, with a
   failed tool call when `maxImages = 0` (no-vision model).

---

## 1. Where the profile field lives (config plumbing)

The OpenAI local-server profile is stored at
`~/.fspec/fspec-config.json → providers.openai.profiles.<name>` (also
deep-merged from `<project>/spec/fspec-config.json`, project-over-user).
Every field added in recent work (PROV-139 streaming, PROV-142
auto-continue, PROV-143 preserve-thinking) flows through the same set of
files. PROV-144 should mirror that exact path:

### Wire shape
- **`rust/rpc-types/src/lib.rs`** — `ProfileDefinition` (line ~476). Add
  `pub max_images: Option<u32>` (flat `Option<u32>` like `context_window`,
  `auto_continue`). Add a canonical predicate
  `max_images_enabled()` / `max_images_limit()` mirroring
  `streaming_enabled()` (line ~512):
  - `None` (key absent on disk) ⇒ **default 4** (new profile behavior).
  - `Some(0)` ⇒ **no vision** (Read tool fails image reads).
  - `Some(n)` n>=1 ⇒ cap of n images per tool result.
- The struct also has a `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`
  projection — adding the field is mechanical; NAPI `index.d.ts` is
  auto-regenerated at build time (no manual TS change required, but the
  `rust/napi/index.d.ts` doc comment block at line ~3445 documents the shape).

### Wire → on-disk bridge
- **`rust/sessions/src/conversions.rs`** — `profile_def_from_wire()`
  (line 165): copy `wire.max_images` into the persistence `ProfileDef`.

### On-disk persistence
- **`rust/sessions/src/profile_persistence.rs`** — `ProfileDef` struct
  (line 35) gains `pub max_images: Option<u32>`; `merge_profile()`
  (line 169) gains a `set_or_remove(profile, "maxImages", def.max_images.map(Value::from))`
  line — the established write-or-remove pattern (absent key = default).
  `save_profile_at`, `rename_profile_at` need no changes (they call
  `merge_profile`).

### Read path (disk → on-disk shape)
- **`rust/sessions/src/profile_sections.rs`** — `LocalServerProfile`
  (line 83) gains a deserialized field
  `#[serde(rename = "maxImages", default, deserialize_with = "de_opt_u32_lenient")]
  pub max_images: Option<u32>` (reusing the existing lenient u32 deserializer,
  line 177, which tolerates floats / oversized numbers so a TS-written config
  never drops the whole profile).

### TUI read-back (prefill)
- **`rust/fspec-tui/src/views/provider_settings/profiles_config.rs`** —
  `profile_definition_from_value()` (line 130) gains a `maxImages` read using
  the same `as_u64 → u32` pattern as `autoContinue` (line 158).

### TUI form
- **`rust/fspec-tui/src/views/provider_settings/profile_form.rs`**:
  - `PROFILE_FORM_FIELDS` (`[&str; 8]`, line 33) → **9 fields**, append
    `"Max Images"` as the 9th (after `Preserve Thinking`).
  - `ProfileForm` struct (line 47) gains `pub max_images: String` (raw typed
    string, parsed on build — same pattern as `auto_continue`).
  - `new_create()` (line 75): seed with `"4"` (the default the user asked for
    — most models support vision; an explicit 4 is written so the profile
    reflects what the form shows).
  - `from_definition()` (line 98): prefill via a small helper
    (`opt_num(def.max_images).or_else(|_| "4".to_string())`-style — absent key
    ⇒ "4", stored value ⇒ its decimal string).
  - `focused_text_mut()` (line 122): add `8 => &mut self.max_images`.
  - `field_value()` (line 181): add `8 => &self.max_images`.
  - `build_definition()` (line 201): parse like `parse_auto_continue` but
    with no rejection (any u32 is valid, including 0). Empty ⇒ `None`
    (⇒ default 4 at resolution time); `"0"` ⇒ `Some(0)`; `"n"` ⇒ `Some(n)`.
    A non-numeric value should reject the save with a hint (mirroring
    `parse_auto_continue`'s `Err(hint)` path) — e.g.
    `"Max Images must be a whole number (0 = no vision, 4 = default)"`.
- **`rust/fspec-tui/src/views/provider_settings/profile_form_parse.rs`**:
  add `parse_max_images(raw) -> Result<Option<u32>, String>` (empty ⇒ None,
  numeric ⇒ Some(n), non-numeric ⇒ Err hint).
- **`rust/fspec-tui/src/views/provider_settings/profile_form_render.rs`**:
  `placeholder_for()` (line 54) add `8 => "4 (default), 0 = no vision"`.
- Navigation (`move_down`/`move_up`) is driven by `PROFILE_FORM_FIELDS.len()`,
  so no routing change is needed; `profile_form_streaming::route_edit_key`
  routes index 8 to the text-editing branch automatically (the boolean-toggle
  guard only special-cases indices 5 and 7).

### Dispatch (unchanged)
- **`rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs`** —
  `SaveProfile` / `rename_profile` dispatch already carries the whole
  `ProfileDefinition`, so no change; the new field rides along.

### NAPI (JS/TS host path)
- **`rust/napi/src/models/napi_bindings.rs`** — `save_profile` (line 360) and
  any rename binding convert via `profile_def_from_wire`, so they pick up the
  new field automatically. No signature change.
- The NAPI surface also re-exports `ProfileDefinition` (index.d.ts); adding a
  field is backward compatible for existing JS callers (they omit it ⇒ None ⇒
  default 4).

---

## 2. Where the limit is resolved (sessions layer)

The established per-session capability plumbing (BUG-168,
`spec/features/session-model-capabilities-registry.feature`) is the exact
pattern to extend:

- **Resolution:** `codelet_sessions::model_resolution::resolve_model_vision`
  (`rust/sessions/src/model_resolution.rs`, line 23). Its **profile branch**
  (lines 28–41) already parses `openai:<profile>/<model>` and reads the
  profile's `customModels[].hasVision` from disk. This is where the **max
  images** value can be read from the same `load_local_server_profiles()`
  lookup (the profile object now carries `max_images` from section 1).
  Propose a sibling function (or a small struct return) — e.g.
  `resolve_profile_max_images(pm) -> Option<u32>` returning `Some(0)` when the
  profile is a profile model with an explicit `maxImages: 0`,
  `Some(n)` when explicit, `None` when (a) not a profile model, or (b) the
  key is absent (⇒ default 4 applied at the tool layer, or by the registry
  writer — see design decision below).

- **Storage:** `codelet_tools::model_capabilities`
  (`rust/tools/src/model_capabilities.rs`). Today it stores a single
  `RwLock<HashMap<Uuid, bool>>` (`supports_vision`). Two options:
  1. **New sibling registry** `SESSION_MODEL_MAX_IMAGES:
     Lazy<RwLock<HashMap<Uuid, u32>>>` with
     `set_session_model_max_images / session_model_max_images` (returns
     `None` when absent ⇒ tool layer applies default 4) — keeps the existing
     boolean registry untouched and matches the "session-registry pattern"
     already used by `done.rs` and `tool_pause`.
  2. Widen the stored value to a struct `{ vision: bool, max_images: Option<u32> }`.
     More invasive; every existing caller (3 set-sites, read-sites in
     `read.rs`, test helpers) changes.
  **Recommendation: option 1** — a sibling registry, same poisoning-graceful
  guard pattern, cleared in the same places (`clear_session_model_vision`
  call-site at `rust/sessions/src/session_manager.rs:1270` and any destroy
  path).

- **Set-sites** (all four call `set_session_model_vision` and must also set
  the new max-images entry):
  1. `rust/sessions/src/session_creation_helper.rs:171` —
     `create_background_session_inner` (shared create path).
  2. `rust/sessions/src/session_manager.rs:1033` —
     `create_isolated_session_with_id`.
  3. `rust/sessions/src/handle_impl.rs:1373` — mid-session `set_model`.
  4. `rust/napi/src/session_bindings.rs:2060` and `:2241` — NAPI
     `session_set_model` and `session_set_model_profile`.

---

## 3. Where the Read tool enforces it

### Image-returning paths in the Read tool
**`rust/tools/src/read.rs`** — the tool returns images in **two shapes**:

| Path | Trigger | Image count |
|---|---|---|
| `ReadOutput::Image` (line 52) | File detected as PNG/JPG/GIF/WEBP (`FileType::Image`) — line 324–327, via `validate_and_encode_image` | **1** image per call (a single file) |
| PDF **visual** mode (`RenderedPdfPages`, `read.rs:411–424`) | `.pdf` with `pdf_mode` default/`"visual"` → `pdf::render_pdf_pages` | **N pages** (offset..limit, capped by `max_pdf_pages()` default 20) |
| PDF **images** mode (`ExtractedPdfImages`, `read.rs:397–410`) | `.pdf` with `pdf_mode: "images"` → `pdf::extract_pdf_images` | **N embedded images** |

SVG is read as text (line 320); text/`ipynb` return text only.

### Enforcement design (matches the user's ask)

The user asked for *either* a message limiting images **or** a hard failure
when the budget is exceeded, with `0` meaning "no vision model" → **fail the
tool call**. Recommended concrete behavior:

1. **At call time** in `ReadTool::call` (and the equivalent
   `FileToolFacadeWrapper` read branch — see below), resolve the session's
   image budget:
   ```
   budget = session_model_max_images(session_id)  // Option<u32>
            .unwrap_or(DEFAULT_MAX_IMAGES /* 4 */);
   ```
   - Registry entry **absent** (unknown session, e.g. a directly-instantiated
     tool, or any non-profile session created before this story) ⇒ default 4
     (the historical "most models support vision" assumption the user
     stated).
   - **`budget == 0`** (no-vision profile): any read that WOULD return an
     image fails:
     - image file (PNG/JPG/GIF/WEBP) ⇒ `ToolError::Validation { message }` —
       e.g. `"Image reading is disabled for this session: the profile 'X'
       has maxImages=0 (no vision). Use text-based tools (e.g. Read with
       pdf_mode='text', Grep) instead of reading image files."`
     - PDF default mode: the existing BUG-168 text-fallback notice already
       handles vision=false sessions; when budget=0 the fallback MUST also
       force **text** mode even when the session capability registry says
       vision=true (an explicit 0 on the profile is the stronger signal —
       it overrides per-model `hasVision`). PDF `"visual"` or `"images"`
       *explicit* modes still fail the call with the same message (the user
       explicitly asked for images; the model can't see them).
   - **`budget >= 1`**:
     - single image file ⇒ always ≤ 1 image ⇒ passes when budget ≥ 1.
     - PDF visual mode ⇒ number of pages returned = `min(limit, budget)`;
       when `budget < requested pages`, the tool call **fails with a message**
       (per the user's "fail the tool call with the message" phrasing):
       `"This session allows at most {budget} image(s) per tool result
       (profile limit). Requested {requested}; use offset/limit to read at
       most {budget} pages, or raise the profile's Max Images setting."`
       Alternative (documented decision point for Example Mapping): clamp to
       `budget` pages and append a truncation notice (like
       `pdf_pagination_notice`, BUG-168 style). The user's wording favors
       **failing with the message**; the clamp-and-notice behavior is the
       existing precedent for PDF pages. **Open question for the PO (see
       §5).**
     - PDF images mode ⇒ same cap on `limit` (number of embedded images).
2. **Where to put the check** so both front doors see it:
   - Primary: `ReadTool::call` in `rust/tools/src/read.rs` — the
     `FileType::Image(media_type)` branch (line 324) and the PDF
     `ExemptFileType::Pdf` branch (line 331) both live there, and
     `self.session_id` is available. This covers rig-native dispatch AND the
     facade path, because `FileToolFacadeWrapper` (`rust/tools/src/facade/wrapper.rs:442`)
     delegates read calls to `ReadTool::call`.
   - The 5MB/pixel-dimension defenses in `validate_and_encode_image`
     (line 61) are a *separate* per-image gate — the new count gate is
     independent (mirroring the BUG-168 layering in
     `spec/features/defense-in-depth-pdf-image-count-cap-in-rig-patch.feature`,
     where the rig-patch count cap is defense-in-depth for *other* tools).
   - Tool description update (`read.rs:250`): add a line such as
     `"Image reads are limited per session (default 4 images per tool
     result; 0 when the profile is configured without vision) — if a read
     exceeds the limit the call fails with the limit and how to adjust it."`

### Error variant
`ToolError::Validation { tool, message }` fits (it's the existing variant for
image-size/pixel-dimension overruns at `read.rs:74` and `:92`). No new
variant needed; the message carries the actionable content (limit value,
offset/limit advice, or "profile is 0 = no vision" guidance).

### Non-profile sessions
Cloud/custom/codex models have no profile ⇒ `resolve_profile_max_images`
returns `None` ⇒ budget default 4 applies uniformly. If a cloud model is
non-vision (BUG-168 registry `vision=false`), the **existing** text-fallback
already degrades PDFs; single-image reads remain allowed (they fail softly at
the provider with `[Image]` placeholders) — out of scope unless Example
Mapping decides to fold `vision=false` ⇒ budget 0 (open question, §5).

---

## 4. Integration point map (files to touch, per layer)

| Layer | File | Change |
|---|---|---|
| wire types | `rust/rpc-types/src/lib.rs` | `ProfileDefinition.max_images: Option<u32>` + predicate |
| wire→disk | `rust/sessions/src/conversions.rs` | `profile_def_from_wire` copy field |
| persistence | `rust/sessions/src/profile_persistence.rs` | `ProfileDef.max_images`, `merge_profile` write/remove `maxImages` |
| disk read | `rust/sessions/src/profile_sections.rs` | `LocalServerProfile.max_images` (lenient deser) |
| resolution | `rust/sessions/src/model_resolution.rs` | new `resolve_profile_max_images` (or extend resolver to return both) |
| capability registry | `rust/tools/src/model_capabilities.rs` | sibling `max_images` registry + set/get/clear |
| session create (shared) | `rust/sessions/src/session_creation_helper.rs` | set registry on create |
| session create (isolated) | `rust/sessions/src/session_manager.rs` | set registry on create (line ~1033) |
| mid-session switch | `rust/sessions/src/handle_impl.rs` | set registry on set_model (line ~1373) |
| NAPI switch | `rust/napi/src/session_bindings.rs` | set registry at lines 2060 / 2241 |
| NAPI save | `rust/napi/src/models/napi_bindings.rs` | none (rides `profile_def_from_wire`) |
| TUI form | `rust/fspec-tui/src/views/provider_settings/profile_form.rs` | field 8: label, struct field, seed, prefill, parse-on-build |
| TUI parse | `rust/fspec-tui/src/views/provider_settings/profile_form_parse.rs` | `parse_max_images` |
| TUI render | `rust/fspec-tui/src/views/provider_settings/profile_form_render.rs` | placeholder at index 8 |
| TUI prefill read | `rust/fspec-tui/src/views/provider_settings/profiles_config.rs` | read `maxImages` key |
| Read tool | `rust/tools/src/read.rs` | budget check in image + PDF branches; description line |
| (defense-in-depth) | `rust/patches/rig-core/.../streaming.rs` | **not changed** — existing 20-page count cap stays independent |
| tests | `rust/tools/tests/`, `rust/sessions/tests/`, `rust/fspec-tui/tests/` | mirror `pdf_vision_fallback_test.rs`, `prov142_auto_continue_persistence.rs`, `provider_settings_profile_form_prov110.rs` patterns |

## 5. Open questions for Example Mapping (red cards)

1. **Exceed-budget behavior for PDFs:** fail the whole tool call (user's
   literal ask: *"a max limit as a message if it does more than that and
   fail the tool call with the message"*) vs. clamp to the budget with a
   "continue with offset=N" notice (existing BUG-168 precedent)? The phrasing
   says **fail**; confirm.
2. **`vision=false` (BUG-168) vs. `maxImages`**: should a resolved
   `vision=false` (non-vision cloud/custom model) *also* force budget 0 for
   image-file reads (not just PDF fallback), or does the budget only come
   from the profile field? Affects non-profile sessions.
3. **Default 4 when key absent vs. "4 only for new profiles":** absent key on
   an *existing* profile — treat as 4 (uniform) or as unlimited (historical
   behavior)? Recommendation: **4** (uniform, matches the form prefill so
   disk and form agree).
4. **Cap range:** allow any `u32` (including huge values) or clamp to a
   ceiling (e.g. 100)? The rig-patch 20-page defense-in-depth cap would still
   apply to *other* tools' PDF-shaped results but not to Read's own output.
5. **Field position in the form:** append after Preserve Thinking (index 8),
   or group near Context Window/Max Output Tokens (the other numeric
   limits)? Recommendation: append (zero routing churn, matches PROV-139/142/143
   "append new fields at the end" history).

## 6. Testing plan sketch

- **Wire:** round-trip `maxImages` JSON (0/4/absent) — pattern of
  `rust/rpc-types/tests/prov142_auto_continue_flag.rs`.
- **Persistence:** save writes/removes `maxImages`; unrelated keys preserved —
  pattern of `rust/sessions/tests/prov142_auto_continue_persistence.rs`.
- **Resolution:** profile with `maxImages: 0` / `7` / absent → registry
  values; mid-session model switch updates registry — pattern of
  `rust/sessions/tests/bug168_model_vision_resolution.rs`.
- **Read tool:**
  - budget 0: image file read fails with "no vision" message; PDF default
    forces text; PDF explicit visual fails.
  - budget 2: PDF with 5 pages + no limit arg → fails (or clamps, per Q1)
    with message naming 2; `limit=2` passes with exactly 2 pages.
  - budget absent (unknown session) → default 4 behavior.
  - Pattern of `rust/tools/tests/pdf_vision_fallback_test.rs`
    (sets `set_session_model_vision`, calls `ReadTool::call` directly with a
    temp-file PDF built via `lopdf` — reuse `create_test_pdf_with_pages`).
- **TUI form:** field present at index 8, placeholder, prefill from stored
  value, empty ⇒ None, "0" ⇒ Some(0), non-numeric rejects save — pattern of
  `rust/fspec-tui/tests/provider_settings_profile_form_prov110.rs` and
  `prov142_auto_continue_form.rs`.

## 7. Risks / notes

- **Two front doors rule:** the check MUST live in `ReadTool::call` (not the
  NAPI/CLI dispatch layers) so the rig-native tool and the
  `FileToolFacadeWrapper`-wrapped facade path both enforce it — same reason
  BUG-168 put the capability check in the tool.
- **300-line file ceiling:** `read.rs` is 456 lines today; the new branch
  logic should be extracted into a small helper (e.g.
  `rust/tools/src/read_image_budget.rs` or folded into
  `model_capabilities.rs`-adjacent module) to keep files under the limit.
- **Env-var bridge (PROV-121):** `apply_profile_env_vars`
  (`model_resolution.rs:224`) bridges `contextWindow`/`streaming` into
  `OPENAI_*` env vars for the provider client. `maxImages` is a *tool-layer*
  concern only (no provider env var needed) — do NOT add it to the env bridge.
- **Backward compatibility:** `maxImages` absent on disk ⇒ default 4; old
  configs keep working. `set_or_remove` guarantees the key is dropped when
  the form clears the field (absent ⇒ default at read time).
- **Windows/cross-platform:** no new I/O; pure in-process registry + JSON
  field — no platform-specific work beyond the existing tests' temp dirs.
