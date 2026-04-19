# vLLM OpenAI-Compatible Server — Research Notes for PROV-081

**Date:** 2026-04-19
**Scope:** Thinking tokens + multimodal (images) over the OpenAI-compatible `/v1/chat/completions` endpoint, as consumed by fspec's OpenAI provider.
**Source:** vLLM cloned to `/tmp/vllm` (`git clone --depth 1 https://github.com/vllm-project/vllm.git`).
**Served model:** `Intel/Qwen3.5-122B-A10B-int4-AutoRound` — a multimodal Qwen3.5 MoE (`Qwen3_5MoeForConditionalGeneration` + `AutoProcessor`, confirmed on HF model card).
**Server launcher:** `vllm.sh` (already configured correctly for reasoning).

---

## 1. Thinking tokens (reasoning content)

### 1.1 Wire format — field name is `reasoning`, NOT `reasoning_content`

This is the gotcha. vLLM chose a different field name than DeepSeek/Z.AI.

**Streaming — `vllm/entrypoints/openai/engine/protocol.py:258-262`:**

```python
class DeltaMessage(OpenAIBaseModel):
    role: str | None = None
    content: str | None = None
    reasoning: str | None = None         # <-- NOT "reasoning_content"
    tool_calls: list[DeltaToolCall] = Field(default_factory=list)
```

**Non-streaming — `vllm/entrypoints/openai/chat_completion/protocol.py:54-64`:**

```python
class ChatMessage(OpenAIBaseModel):
    role: str
    content: str | None = None
    refusal: str | None = None
    ...
    # vLLM-specific fields that are not in OpenAI spec
    reasoning: str | None = None
```

So a streaming chunk looks like:

```json
{"choices":[{"delta":{"reasoning":"Let me analyse…"}}]}
{"choices":[{"delta":{"reasoning":" the user is asking…"}}]}
{"choices":[{"delta":{"content":"The answer is …"}}]}
```

And a non-streaming response:

```json
{"choices":[{"message":{"role":"assistant",
                         "reasoning":"Let me analyse…",
                         "content":"The answer is …"}}]}
```

> Note: On the **input** side (prior assistant messages fed back for interleaved thinking), `chat_utils.py:1544-1548` accepts **both** `reasoning` and `reasoning_content` for compatibility. But the **output** only ever uses `reasoning`.

### 1.2 The server must be started with `--reasoning-parser`

If `--reasoning-parser` is unset, vLLM emits the raw `<think>…</think>` inline as content. `vllm/entrypoints/openai/chat_completion/serving.py:129-131`:

```python
self.reasoning_parser_cls = ParserManager.get_reasoning_parser(
    reasoning_parser_name=reasoning_parser
)
```

**Our `vllm.sh:408` already does this:** `--reasoning-parser qwen3` ✅. No server change needed for thinking.

Available parser names live under `/tmp/vllm/vllm/reasoning/`:
`deepseek_r1`, `deepseek_v3`, `ernie45`, `gemma4`, `gptoss`, `granite`, `hunyuan_a13b`,
`identity`, `kimi_k2`, `minimax_m2`, `mistral`, `nemotron_v3`, `olmo3`, `qwen3`,
`seedoss`, `step3`, `step3p5`.

### 1.3 Qwen3 parser behavior — `vllm/reasoning/qwen3_reasoning_parser.py`

```python
class Qwen3ReasoningParser(BaseThinkingReasoningParser):
    def __init__(self, tokenizer, *args, **kwargs):
        super().__init__(tokenizer, *args, **kwargs)
        chat_kwargs = kwargs.get("chat_template_kwargs", {}) or {}
        # Qwen3 defaults to thinking enabled; only treat output as
        # pure content when the user explicitly disables it.
        self.thinking_enabled = chat_kwargs.get("enable_thinking", True)

    @property
    def start_token(self) -> str: return "<think>"
    @property
    def end_token(self) -> str:   return "</think>"
```

Key behaviors:

- `enable_thinking` defaults to **True** (line 42). Don't send `false` or thinking dies at the template level.
- Starting with Qwen3.5, the chat template **pre-places** `<think>` in the prompt, so only `</think>` appears in generated output. The parser handles both old- and new-style templates.
- When thinking is off AND no `</think>` appears, everything is treated as content. When thinking is on and no `</think>` appears (truncation), everything is treated as reasoning.

### 1.4 `include_reasoning` request flag

`chat_completion/protocol.py:184`:

```python
include_reasoning: bool = True
```

If the client sends `false`, vLLM actively strips reasoning from both streaming and non-streaming paths (see `serving.py:320-321`, `:1316-1317`, `:1371-1372`). **Default `true` is correct — don't override.**

### 1.5 Streaming decision tree

Relevant block: `chat_completion/serving.py:848-878`.

1. If chat template set `enable_thinking=False` (detected via `prompt_is_reasoning_end`), the parser is **never** called and everything streams as `{content: …}`.
2. Otherwise the parser is called per-delta and emits `{reasoning: …}` until `</think>` is observed, then switches to `{content: …}`.
3. Note the parser is tolerant of the old template style — if `<think>` appears in the generated output itself it is stripped before emission.

---

## 2. Multimodal (images)

### 2.1 The model IS multimodal

HF card for `Intel/Qwen3.5-122B-A10B-int4-AutoRound` (https://huggingface.co/Intel/Qwen3.5-122B-A10B-int4-AutoRound):

```python
from transformers import AutoProcessor, Qwen3_5MoeForConditionalGeneration
model = Qwen3_5MoeForConditionalGeneration.from_pretrained(model_name, ...)
processor = AutoProcessor.from_pretrained(model_name)

messages = [{
    "role": "user",
    "content": [
        {"type": "image", "image": "https://.../demo.jpeg"},
        {"type": "text",  "text": "Describe this image in short."},
    ],
}]
```

`Qwen3_5MoeForConditionalGeneration` + `AutoProcessor` unambiguously indicates a vision-capable model. The HF-native content format uses `{"type": "image", "image": "<url>"}`.

### 2.2 vLLM's OpenAI-compatible image input

vLLM translates the **OpenAI** image format (not the HF-native one) before handing off to the processor. From `/tmp/vllm/examples/online_serving/openai_chat_completion_client_for_multimodal.py:82-97`:

```python
chat_completion_from_url = client.chat.completions.create(
    messages=[{
        "role": "user",
        "content": [
            {"type": "text", "text": "What's in this image?"},
            {"type": "image_url", "image_url": {"url": image_url}},
        ],
    }],
    model=model,
)
```

### 2.3 Accepted content-part `type` values

From `/tmp/vllm/vllm/entrypoints/chat_utils.py:1267-1325` + `1455-1498`:

| Type                                | Payload shape                                                | Notes                                    |
|-------------------------------------|--------------------------------------------------------------|------------------------------------------|
| `text` / `input_text` / `output_text` / `refusal` / `thinking` | `{type, text}`                                               | Treated as text                          |
| `image_url` (canonical)             | `{type, image_url: {url: "…"}}`                              | URL, data URL, or `file://` (see 2.4)    |
| `image_url` (simplified)            | `{type, image_url: "…"}`                                     | String form, OpenAI undocumented variant |
| `input_image`                       | `{type, image_url: "…"}`                                     | Alias from OpenAI Responses API          |
| `image_pil`                         | `{type, image_pil: <PIL.Image>}`                             | In-process only                          |
| `image_embeds`                      | `{type, image_embeds: "<b64>" | {…}}`                        | Pre-computed tensor                      |
| `audio_url`                         | `{type, audio_url: {url: "…"}}`                              |                                          |
| `input_audio`                       | `{type, input_audio: {…}}`                                   |                                          |
| `audio_embeds`                      | `{type, audio_embeds: "<b64>"}`                              |                                          |
| `video_url`                         | `{type, video_url: {url: "…"}}`                              |                                          |

Each media part may optionally carry a `"uuid": "<client-side-id>"` for dedup across requests.

**❌ HF-native `{"type": "image", "image": "…"}` is NOT accepted by vLLM's OpenAI endpoint** — that form is only valid in the `transformers` direct path. The whitelist lives in `chat_utils.py:1473-1480`:

```python
elif part_type in ("image_url", "input_image"):
    ...
```

### 2.4 URL schemes accepted for `image_url.url`

- `https://…` / `http://…` — fetched by the server
- `data:image/<type>;base64,<base64>` — inline
- `file:///abs/path` — **only if** server launched with `--allowed-local-media-path /allowed/dir`

### 2.5 Server-side `--limit-mm-per-prompt.image N`

vLLM's default is **1 image per prompt**. To send multiple images the server must be launched with `--limit-mm-per-prompt.image N`. From `openai_chat_completion_client_for_multimodal.py:13`:

```
vllm serve microsoft/Phi-3.5-vision-instruct --runner generate \
    --trust-remote-code --max-model-len 4096 --limit-mm-per-prompt.image 2
```

**Our `vllm.sh` does NOT currently pass this flag.** The `docker run` invocation in `start()` (lines 398-417) ends at `--speculative-config '{"method":"mtp","num_speculative_tokens":2}'` — no multimodal limits. Appending `--limit-mm-per-prompt.image 4` (or similar) is a one-line server fix required for multi-image scenarios.

---

## 3. Client-side gap analysis (fspec)

### 3.1 Streaming path — misses `reasoning`

`codelet/patches/rig-core/src/providers/openai/completion/streaming.rs:36-44`:

```rust
#[derive(Deserialize, Debug)]
struct StreamingDelta {
    #[serde(default)]
    content: Option<String>,
    /// Z.AI GLM models send reasoning/thinking content in this field
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default, deserialize_with = "json_utils::null_or_vec")]
    tool_calls: Vec<StreamingToolCall>,
}
```

Usage at `streaming.rs:298-302`:

```rust
if let Some(reasoning) = &delta.reasoning_content && !reasoning.is_empty() {
    ...
        reasoning: reasoning.clone(),
    ...
```

There is no `reasoning` field, so vLLM's reasoning stream is deserialized into serde's ignored-extras bucket and silently lost.

### 3.2 Non-streaming path — same gap

`codelet/patches/rig-core/src/providers/openai/completion/mod.rs` references `reasoning_content` and `completion_tokens_details.reasoning_tokens`, but not `message.reasoning`. Needs a mirror fix.

### 3.3 Image content parts — not constructed

Searches for `image_url` in `codelet/patches/rig-core/src/providers/openai/` return only type stubs, not constructors that wrap image inputs into OpenAI-compatible parts. The message-to-request translation path doesn't produce `{type:"image_url", image_url:{url:…}}` parts.

(Work unit for images tracked alongside thinking in PROV-081 now that the model's multimodal capability is confirmed.)

---

## 4. Minimum fix plan

### 4.1 Client

1. **Streaming** (`streaming.rs:36-44`): add `#[serde(alias = "reasoning")] reasoning_content: Option<String>` OR add a separate `reasoning: Option<String>` field and OR them together at `:298-302`. Serde `alias` is cleaner because it avoids duplicate emission logic.
2. **Non-streaming** (`completion/mod.rs`): same treatment for `ChatMessage`.
3. **Do not send** `include_reasoning: false` on the request.
4. **Do not send** `chat_template_kwargs: {enable_thinking: false}` on Qwen3 requests.
5. **Image content-part constructor**: wrap image inputs as
   `{"type":"image_url","image_url":{"url":"<https|data|file url>"}}`
   — never as `{"type":"image","image":"…"}` (that's HF-native, rejected by vLLM's OpenAI endpoint).

### 4.2 Server (`vllm.sh`)

1. Append `--limit-mm-per-prompt.image 4` (or a chosen N) to the `docker run … serve …` line in `start()` (around `vllm.sh:417`). Without this, only one image per request works.
2. No other changes. `--reasoning-parser qwen3` (`:408`), `--tool-call-parser qwen3_xml` (`:415`), `--enable-auto-tool-choice` (`:414`) are correct.

---

## 5. Acceptance quick-check

Once both fixes are in:

```bash
curl http://DGX:8000/v1/chat/completions -s -H 'content-type: application/json' -d '{
  "model": "qwen",
  "stream": true,
  "messages": [{
    "role": "user",
    "content": [
      {"type":"text","text":"What is in this image?"},
      {"type":"image_url","image_url":{"url":"https://vllm-public-assets.s3.us-west-2.amazonaws.com/vision_model_images/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg"}}
    ]
  }]
}' | grep -o '"reasoning":"[^"]*"' | head -3
```

Expected: at least a few `"reasoning":"…"` chunks before the first `"content":"…"` chunk.

---

## 6. References (files read in `/tmp/vllm`)

- `vllm/entrypoints/openai/engine/protocol.py` — `DeltaMessage`
- `vllm/entrypoints/openai/chat_completion/protocol.py` — `ChatMessage`, `ChatCompletionRequest`, `include_reasoning`, `chat_template_kwargs`
- `vllm/entrypoints/openai/chat_completion/serving.py` — reasoning/tool parser wiring, streaming dispatch
- `vllm/reasoning/basic_parsers.py` — `BaseThinkingReasoningParser`
- `vllm/reasoning/qwen3_reasoning_parser.py` — Qwen3 specifics
- `vllm/entrypoints/chat_utils.py` — multimodal part parsing (`image_url`, `image_pil`, `image_embeds`, `audio_url`, `video_url`, whitelist at `:1473-1480`)
- `examples/online_serving/openai_chat_completion_client_for_multimodal.py` — canonical image payload
- `vllm/engine/arg_utils.py` — `--reasoning-parser` CLI flag definition

HF: https://huggingface.co/Intel/Qwen3.5-122B-A10B-int4-AutoRound (confirmed multimodal via `Qwen3_5MoeForConditionalGeneration` + `AutoProcessor`).
