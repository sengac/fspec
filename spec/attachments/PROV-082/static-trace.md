# PROV-082 Static Trace — image input → OpenAI Chat Completions provider

Traces ALL code paths by which an image can enter the OpenAI Chat Completions
provider in codelet, and identifies where each path breaks.

## Legend
- `[OK]` path works end-to-end
- `[BREAK:a]` base64 image conversion rejected because detail is None
- `[BREAK:b]` tool-result image content rejected because provider only handles text

---

## Path 1 — User base64 image via Telegram / bridge paste

Trigger:   User sends an image over a bridge (Telegram) and it arrives in NAPI
           via IncomingMessage.with_images()

1. codelet/napi/src/session_manager.rs:5318-5322
   BridgeImageData → codelet_cli::interactive::BridgeImage

2. codelet/napi/src/session_manager.rs:4335
   run_agent_stream_with_images(.., images, ..)

3. codelet/cli/src/interactive/stream_loop.rs:211-239
   pub fn run_agent_stream_with_images(agent, prompt, images: Option<Vec<BridgeImage>>, ..)

4. codelet/cli/src/interactive/stream_loop.rs:469-471
   session.messages.push(Message::User {
       content: build_user_content_with_images(effective_prompt, images),
   });

5. codelet/cli/src/interactive/multimodal.rs:63        <-- detail is ALWAYS None
   content_parts.push(UserContent::image_base64(img.data, media_type, None));

6. codelet/core/src/rig_agent.rs:171-178
   self.agent.stream_prompt(prompt).with_history(history_for_rig)...
   (history includes the Message::User with UserContent::Image(base64))

7. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:1077
   partial_history.into_iter().map(message::Message::try_into).collect(...)?

8. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:584-592
   TryFrom<message::Message> for Vec<Message>
     → content.try_into()  (OneOrMany<UserContent> → Vec<Message>)

9. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:507-540
   TryFrom<OneOrMany<message::UserContent>> for Vec<Message>
     → for each UserContent, content.try_into()

10. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:416-504
    TryFrom<message::UserContent> for UserContent (OpenAI)
     → message::UserContent::Image branch at line 422-462
     → DocumentSourceKind::Base64 branch at 434-452

11. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:445-447
     let detail = detail.ok_or(message::MessageError::ConversionError(
         "OpenAI image URI must have image detail".into(),
     ))?;

    ### [BREAK:a]  ⚠️  detail is None (step 5), ok_or returns ConversionError.

12. codelet/patches/rig-core/src/completion/message.rs:1081
    impl From<MessageError> for CompletionError { ... }

    Error bubbles back through .try_into() → CompletionRequest::try_from(...)
    → StreamingCompletionModel::stream(...) as a CompletionError::Conversion.

RESULT:
  The HTTP request is never constructed. No /chat/completions call is made.
  The TUI/bridge surfaces a conversion error. From the user's perspective
  the image "never reaches vLLM" — true, because the provider short-circuits.

---

## Path 2 — Tool-returned image (Read tool on a PNG/JPG file)

Trigger:  Assistant calls Read("path/to/image.png")

1. codelet/tools/src/read.rs:324-326
   FileType::Image(media_type) ⇒ validate_and_encode_image(...)

2. codelet/tools/src/read.rs:101-104
   Ok(ReadOutput::Image { data: base64_data, media_type: media_type.as_mime() })

3. codelet/tools/src/read.rs:46-53
   #[derive(Serialize)]
   #[serde(tag = "type", rename_all = "lowercase")]
   pub enum ReadOutput { Text {...}, Image { data, media_type } }

4. codelet/tools/src/read.rs:409-412
   serde_json::to_string(&output)  // emits {"type":"image","data":"...","media_type":"image/png"}

5. rig agent loop receives `String` tool output:
   codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:685
   let tr = ToolResult { id, call_id, content: vec_to_one_or_many(parse_tool_result_content(&text)) };

6. codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:182-203
   parse_tool_result_content detects type == "image" and returns
     vec![ ToolResultContent::image_base64(data, Some(media_type), None) ]

7. streaming.rs pushes it back into chat_history as
   UserContent::ToolResult( ToolResult { content: OneOrMany<ToolResultContent::Image(...)> } )

8. Next turn: codelet/patches/rig-core/src/providers/openai/completion/mod.rs:584-592
   TryFrom<message::Message>::try_into()

9. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:510-524
   TryFrom<OneOrMany<message::UserContent>> for Vec<Message>
     partitions UserContent::ToolResult → tool_results.map(try_into)

10. codelet/patches/rig-core/src/providers/openai/completion/mod.rs:393-413
    impl TryFrom<message::ToolResult> for Message  (OpenAI Chat Completions)
        .map(|content| match content {
            message::ToolResultContent::Text(...) => Ok(text),
            _ => Err(MessageError::ConversionError(
                "Tool result content does not support non-text".into(),
            )),
        })

    ### [BREAK:b]  ⚠️  ToolResultContent::Image hits the wildcard arm, conversion fails.

RESULT:
  Conversion error before any HTTP request is built. vLLM never sees the
  image from a Read tool call.

  Contrast: codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs:295-322
  DOES correctly handle ToolResultContent::Image for the OpenAI Responses
  API by emitting an InputImage content item. But the codelet OpenAI
  provider (codelet/providers/src/openai.rs:238) constructs
  openai::completion::CompletionModel — the Chat Completions variant — not
  the Responses API variant. So codelet → vLLM always takes the broken
  completion::TryFrom<message::ToolResult> path.

---

## Path 3 — User URL image via bridge

(For completeness — this path works.)

1. (same as Path 1 through step 4)
2. codelet/cli/src/interactive/multimodal.rs would need to call
   UserContent::image_url(...) — but actually doesn't, see Gap below.
3. completion/mod.rs:428-433 — DocumentSourceKind::Url branch
     detail: detail.unwrap_or_default(),     <-- defaults to Auto when None

RESULT:  `[OK]`   If anything ever called UserContent::image_url, it works.

---

## Path 4 — CLI TUI "normal" user chat (no bridge)

Trigger:  User types a prompt in the CLI TUI; no image attachment mechanism
          exists in codelet/tui/src/events.rs (only Key, Paste string, Draw,
          Resize are surfaced — no image paste, no file attach).

codelet/cli/src/lib.rs:146,152,158,164
    run_agent_stream(agent, prompt).await        <-- no images parameter

codelet/cli/src/interactive/stream_loop.rs:177-204
    run_agent_stream(...) calls run_agent_stream_internal(.., images: None, ..)

codelet/cli/src/interactive/multimodal.rs:29,68
    match images { Some(..) => {..} None => OneOrMany::one(UserContent::text(prompt)) }

RESULT:  TUI/CLI user CANNOT attach an image inline. Only the Read tool
         path (Path 2) can introduce images into the conversation for
         ordinary users — and that path is broken by [BREAK:b].

---

## Summary of breaks

| Path | Trigger                              | Status  | Break point                                    |
|------|--------------------------------------|---------|------------------------------------------------|
| 1    | Bridge base64 image (Telegram etc.)  | BREAK:a | openai/completion/mod.rs:445-447               |
| 2    | Read tool returns Image              | BREAK:b | openai/completion/mod.rs:400-404               |
| 3    | Bridge URL image (hypothetical)      | OK      | Works via DocumentSourceKind::Url path         |
| 4    | CLI/TUI inline image                 | N/A     | No inline image input surface in codelet       |

## Where the OpenAI provider creates its model

codelet/providers/src/openai.rs:216-247
  let rig_client = openai::CompletionsClient::builder().base_url(url).build()?;
  let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);

This unambiguously uses the Chat Completions code path (mod.rs), NOT the
Responses API (responses_api/mod.rs). Chat Completions is what vLLM serves
at /v1/chat/completions.
