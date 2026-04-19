# PROV-082 Findings — Multimodal image delivery to vLLM

## Executive summary

Images NEVER reach vLLM from codelet via the OpenAI Chat Completions provider
— not because the wire payload is malformed, but because **the rig-core OpenAI
Chat Completions provider returns a `MessageError::ConversionError` before the
HTTP request is even built**. Two distinct conversion breaks exist, both
confirmed live via the PROV-082 wire-capture harness:

1. **Break A (`completion/mod.rs:445-447`)** — base64 `UserContent::Image` with
   `detail: None` is rejected.  Every base64 image created in codelet passes
   `detail = None`.
2. **Break B (`completion/mod.rs:400-404`)** — `ToolResultContent::Image`
   (returned from `Read` on a PNG/JPG, or from PDF visual mode) is rejected
   because the provider's `TryFrom<message::ToolResult>` only accepts `Text`.

Server side is already confirmed fine (PROV-082 context). The wire payload
IS correctly shaped WHEN the conversion succeeds (see harness Scenarios C/D).

## Root-cause classification

**5. Something else** — specifically: two independent rig-core OpenAI Chat
Completions provider conversion errors in `codelet/patches/rig-core/`. This is
NOT classification #1 (server-side multi-image limit), NOT #2 (codelet never
builds `UserContent::Image` — it does), NOT #3 (Responses vs Chat API — we
verified codelet uses Chat Completions, which is correct for vLLM), and NOT
#4 (ConversionError is not silently dropped — it IS propagated, but it still
means no image ever hits the wire).

## Evidence

### Static trace (file:line)

- Base64 user image: built with `detail = None` at
  `codelet/cli/src/interactive/multimodal.rs:63`
  ```rust
  content_parts.push(UserContent::image_base64(img.data, media_type, None));
  ```

- Base64 conversion error path:
  `codelet/patches/rig-core/src/providers/openai/completion/mod.rs:445-447`
  ```rust
  let detail = detail.ok_or(message::MessageError::ConversionError(
      "OpenAI image URI must have image detail".into(),
  ))?;
  ```

- Read tool → image tool result: `codelet/tools/src/read.rs:101-104` emits
  `ReadOutput::Image { data, media_type }` serialized as JSON
  `{"type":"image","data":"...","media_type":"image/png"}`.

- Rig parses that into `ToolResultContent::image_base64(data, Some(mt), None)`
  at `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:200`.

- Tool-result image rejection:
  `codelet/patches/rig-core/src/providers/openai/completion/mod.rs:400-404`
  ```rust
  .map(|content| match content {
      message::ToolResultContent::Text(message::Text { text }) => Ok(text),
      _ => Err(message::MessageError::ConversionError(
          "Tool result content does not support non-text".into(),
      )),
  })
  ```

- Codelet uses the Chat Completions client (not Responses API):
  `codelet/providers/src/openai.rs:216-238`
  ```rust
  let rig_client = openai::CompletionsClient::builder().base_url(url).build()?;
  let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);
  ```

- For contrast, the Responses API path DOES handle tool-result images
  correctly: `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs:295-322`.

### Wire capture (Phase 2)

Throwaway harness at
`codelet/patches/rig-core/examples/prov_082_wire_capture.rs` drives each
scenario through `<Vec<openai::Message> as TryFrom<rig::Message>>::try_from`:

```
--- Scenario A: USER base64 image with detail=None (bridge path) ---
FAILED conversion: Message conversion error: OpenAI image URI must have image detail

--- Scenario B: TOOL result with ToolResultContent::Image (Read tool path) ---
FAILED conversion: Message conversion error: Tool result content does not support non-text

--- Scenario C: USER URL image (for comparison, DOES work) ---
SUCCESS — ... "type":"image_url","image_url":{"url":"https://...","detail":"auto"}

--- Scenario D: USER base64 image with detail=Some(Auto) — the FIX ---
SUCCESS — ... "type":"image_url","image_url":{"url":"data:image/png;base64,...","detail":"auto"}
```

Full capture logged to `spec/attachments/PROV-082/request-body.txt`.

### TUI/CLI gap

`codelet/tui/src/events.rs:13-22` only surfaces `Key`, `Paste(String)`,
`Draw`, `Resize` — there is NO image input surface for TUI/CLI users. Image
paths exist ONLY for:
- Telegram / bridge paste (Path 1, broken by Break A)
- Read tool results (Path 2, broken by Break B)
- MCP tools returning `{"type":"image"}` (Path 2, same break)

See `spec/attachments/PROV-082/static-trace.md` for the full path-by-path
trace.

## Does ~/vllm.sh need `--limit-mm-per-prompt.image N`?

**Irrelevant for the primary symptom.** No image ever reaches vLLM in the
current codelet build, so the server-side multi-image limit is moot. If/when
the fix is applied and images start flowing, then:
- Single-image use (most common — user reads one screenshot, one diagram):
  not needed (default is 1).
- Multi-image use (user pastes several images, or PDF visual mode with >1
  page): will need `--limit-mm-per-prompt.image N` where N ≥ image count.

The `--limit-mm-per-prompt` flag should be documented in the deployment
runbook but is not part of PROV-082's scope (out of scope explicitly, and
$HOME/vllm.sh is not in the repo).

## Recommended follow-up work units

### PROV-083 (bug)
**Title:** OpenAI Chat Completions provider rejects base64 user images with detail=None

**Scope:** Fix `codelet/patches/rig-core/src/providers/openai/completion/mod.rs:445-447`
to default missing `ImageDetail` to `ImageDetail::default()` (Auto), matching
the URL path at line 431. Alternatively, update
`codelet/cli/src/interactive/multimodal.rs:63` to pass
`Some(ImageDetail::Auto)`, but fixing at the provider is safer (applies to
every caller, including NAPI session_manager, tests, and future entry
points).

**Rationale:** No sensible OpenAI-compatible server treats missing
`detail` as an error (OpenAI itself defaults to `auto`). The strict
`ok_or` is a latent bug copied from upstream rig-core — upstream has no
caller that uses base64 + None, but codelet does.

### PROV-084 (bug)
**Title:** OpenAI Chat Completions provider drops tool-returned images
(Read tool, PDF visual mode, MCP image outputs)

**Scope:** Extend `codelet/patches/rig-core/src/providers/openai/completion/mod.rs:393-414`
(`TryFrom<message::ToolResult> for Message`) to handle
`ToolResultContent::Image` by:
- EITHER splitting the tool result into an OpenAI `tool` message for text
  parts + a follow-up `user` message with `image_url` content for image
  parts (matches the pattern used by the Responses API path at
  `responses_api/mod.rs:295-322` and by Anthropic at
  `anthropic/completion.rs:485-499`);
- OR returning a multi-message `Vec<Message>` from the conversion, which
  requires upgrading the `TryFrom` target type.

The Chat Completions spec does not support image content inside a `tool`
role message directly, so the split-into-user-follow-up is the only
OpenAI-compliant option. vLLM accepts this pattern — verified by the
server-side curl in the task context.

### Dependencies
- PROV-083 blocks PROV-084 (Break A fires first when any image is present;
  Break B fires on the Read tool path specifically).
- Both blocked by PROV-082 (this spike).

### Optional: codelet/tui image input surface
If users expect to paste images directly in the TUI (not via bridge), a
separate work unit for TUI multimodal input (clipboard image → base64 →
`UserContent::image_base64`) would be required. Out of scope for PROV-082.

## Artifacts

- `spec/attachments/PROV-082/findings.md` — this report
- `spec/attachments/PROV-082/static-trace.md` — full path-by-path trace
- `spec/attachments/PROV-082/request-body.txt` — wire capture output
- `codelet/patches/rig-core/examples/prov_082_wire_capture.rs` — harness
  (throwaway, can be deleted when PROV-083/PROV-084 are closed)
