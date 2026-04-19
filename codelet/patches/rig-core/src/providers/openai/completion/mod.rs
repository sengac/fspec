// ================================================================
// OpenAI Completion API
// ================================================================

use super::{
    CompletionsClient as Client,
    client::{ApiErrorResponse, ApiResponse},
    streaming::StreamingCompletionResponse,
};
use crate::completion::{
    CompletionError, CompletionRequest as CoreCompletionRequest, GetTokenUsage,
};
use crate::http_client::{self, HttpClientExt};
use crate::message::{AudioMediaType, DocumentSourceKind, ImageDetail, MimeType};
use crate::one_or_many::string_or_one_or_many;
use crate::telemetry::{ProviderResponseExt, SpanCombinator};
use crate::wasm_compat::{WasmCompatSend, WasmCompatSync};
use crate::{OneOrMany, completion, json_utils, message};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fmt;
use tracing::{Instrument, Level, enabled, info_span};

use std::str::FromStr;

pub mod streaming;

/// `gpt-5.1` completion model
pub const GPT_5_1: &str = "gpt-5.1";

/// `gpt-5` completion model
pub const GPT_5: &str = "gpt-5";
/// `gpt-5` completion model
pub const GPT_5_MINI: &str = "gpt-5-mini";
/// `gpt-5` completion model
pub const GPT_5_NANO: &str = "gpt-5-nano";

/// `gpt-4.5-preview` completion model
pub const GPT_4_5_PREVIEW: &str = "gpt-4.5-preview";
/// `gpt-4.5-preview-2025-02-27` completion model
pub const GPT_4_5_PREVIEW_2025_02_27: &str = "gpt-4.5-preview-2025-02-27";
/// `gpt-4o-2024-11-20` completion model (this is newer than 4o)
pub const GPT_4O_2024_11_20: &str = "gpt-4o-2024-11-20";
/// `gpt-4o` completion model
pub const GPT_4O: &str = "gpt-4o";
/// `gpt-4o-mini` completion model
pub const GPT_4O_MINI: &str = "gpt-4o-mini";
/// `gpt-4o-2024-05-13` completion model
pub const GPT_4O_2024_05_13: &str = "gpt-4o-2024-05-13";
/// `gpt-4-turbo` completion model
pub const GPT_4_TURBO: &str = "gpt-4-turbo";
/// `gpt-4-turbo-2024-04-09` completion model
pub const GPT_4_TURBO_2024_04_09: &str = "gpt-4-turbo-2024-04-09";
/// `gpt-4-turbo-preview` completion model
pub const GPT_4_TURBO_PREVIEW: &str = "gpt-4-turbo-preview";
/// `gpt-4-0125-preview` completion model
pub const GPT_4_0125_PREVIEW: &str = "gpt-4-0125-preview";
/// `gpt-4-1106-preview` completion model
pub const GPT_4_1106_PREVIEW: &str = "gpt-4-1106-preview";
/// `gpt-4-vision-preview` completion model
pub const GPT_4_VISION_PREVIEW: &str = "gpt-4-vision-preview";
/// `gpt-4-1106-vision-preview` completion model
pub const GPT_4_1106_VISION_PREVIEW: &str = "gpt-4-1106-vision-preview";
/// `gpt-4` completion model
pub const GPT_4: &str = "gpt-4";
/// `gpt-4-0613` completion model
pub const GPT_4_0613: &str = "gpt-4-0613";
/// `gpt-4-32k` completion model
pub const GPT_4_32K: &str = "gpt-4-32k";
/// `gpt-4-32k-0613` completion model
pub const GPT_4_32K_0613: &str = "gpt-4-32k-0613";

/// `o4-mini-2025-04-16` completion model
pub const O4_MINI_2025_04_16: &str = "o4-mini-2025-04-16";
/// `o4-mini` completion model
pub const O4_MINI: &str = "o4-mini";
/// `o3` completion model
pub const O3: &str = "o3";
/// `o3-mini` completion model
pub const O3_MINI: &str = "o3-mini";
/// `o3-mini-2025-01-31` completion model
pub const O3_MINI_2025_01_31: &str = "o3-mini-2025-01-31";
/// `o1-pro` completion model
pub const O1_PRO: &str = "o1-pro";
/// `o1`` completion model
pub const O1: &str = "o1";
/// `o1-2024-12-17` completion model
pub const O1_2024_12_17: &str = "o1-2024-12-17";
/// `o1-preview` completion model
pub const O1_PREVIEW: &str = "o1-preview";
/// `o1-preview-2024-09-12` completion model
pub const O1_PREVIEW_2024_09_12: &str = "o1-preview-2024-09-12";
/// `o1-mini completion model
pub const O1_MINI: &str = "o1-mini";
/// `o1-mini-2024-09-12` completion model
pub const O1_MINI_2024_09_12: &str = "o1-mini-2024-09-12";

/// `gpt-4.1-mini` completion model
pub const GPT_4_1_MINI: &str = "gpt-4.1-mini";
/// `gpt-4.1-nano` completion model
pub const GPT_4_1_NANO: &str = "gpt-4.1-nano";
/// `gpt-4.1-2025-04-14` completion model
pub const GPT_4_1_2025_04_14: &str = "gpt-4.1-2025-04-14";
/// `gpt-4.1` completion model
pub const GPT_4_1: &str = "gpt-4.1";

impl From<ApiErrorResponse> for CompletionError {
    fn from(err: ApiErrorResponse) -> Self {
        CompletionError::ProviderError(err.message)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    #[serde(alias = "developer")]
    System {
        #[serde(deserialize_with = "string_or_one_or_many")]
        content: OneOrMany<SystemContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    User {
        #[serde(deserialize_with = "string_or_one_or_many")]
        content: OneOrMany<UserContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        #[serde(default, deserialize_with = "json_utils::string_or_vec")]
        content: Vec<AssistantContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        audio: Option<AudioAssistant>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// PROV-081: capture vLLM-native `reasoning` reasoning/thinking text on
        /// non-streaming assistant messages. Aliased so that Z.AI/GLM payloads
        /// carrying `reasoning_content` are also captured. The outbound
        /// serialization skips this field (assistant messages we send upstream
        /// never need to carry reasoning back — the panic at AssistantContent::
        /// Reasoning elsewhere in this module guards that invariant).
        #[serde(
            default,
            alias = "reasoning_content",
            skip_serializing_if = "Option::is_none"
        )]
        reasoning: Option<String>,
        #[serde(
            default,
            deserialize_with = "json_utils::null_or_vec",
            skip_serializing_if = "Vec::is_empty"
        )]
        tool_calls: Vec<ToolCall>,
    },
    #[serde(rename = "tool")]
    ToolResult {
        tool_call_id: String,
        content: ToolResultContentValue,
    },
}

impl Message {
    pub fn system(content: &str) -> Self {
        Message::System {
            content: OneOrMany::one(content.to_owned().into()),
            name: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AudioAssistant {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SystemContent {
    #[serde(default)]
    pub r#type: SystemContentType,
    pub text: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SystemContentType {
    #[default]
    Text,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AssistantContent {
    Text { text: String },
    Refusal { refusal: String },
}

impl From<AssistantContent> for completion::AssistantContent {
    fn from(value: AssistantContent) -> Self {
        match value {
            AssistantContent::Text { text } => completion::AssistantContent::text(text),
            AssistantContent::Refusal { refusal } => completion::AssistantContent::text(refusal),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum UserContent {
    Text {
        text: String,
    },
    #[serde(rename = "image_url")]
    Image {
        image_url: ImageUrl,
    },
    Audio {
        input_audio: InputAudio,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ImageUrl {
    pub url: String,
    #[serde(default)]
    pub detail: ImageDetail,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct InputAudio {
    pub data: String,
    pub format: AudioMediaType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ToolResultContent {
    #[serde(default)]
    r#type: ToolResultContentType,
    pub text: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolResultContentType {
    #[default]
    Text,
}

impl FromStr for ToolResultContent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_owned().into())
    }
}

impl From<String> for ToolResultContent {
    fn from(s: String) -> Self {
        ToolResultContent {
            r#type: ToolResultContentType::default(),
            text: s,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ToolResultContentValue {
    Array(Vec<ToolResultContent>),
    String(String),
}

impl ToolResultContentValue {
    pub fn from_string(s: String, use_array_format: bool) -> Self {
        if use_array_format {
            ToolResultContentValue::Array(vec![ToolResultContent::from(s)])
        } else {
            ToolResultContentValue::String(s)
        }
    }

    pub fn as_text(&self) -> String {
        match self {
            ToolResultContentValue::Array(arr) => arr
                .iter()
                .map(|c| c.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            ToolResultContentValue::String(s) => s.clone(),
        }
    }

    pub fn to_array(&self) -> Self {
        match self {
            ToolResultContentValue::Array(_) => self.clone(),
            ToolResultContentValue::String(s) => {
                ToolResultContentValue::Array(vec![ToolResultContent::from(s.clone())])
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(default)]
    pub r#type: ToolType,
    pub function: Function,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Function,
}

/// Function definition for a tool, with optional strict mode
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

impl From<completion::ToolDefinition> for ToolDefinition {
    fn from(tool: completion::ToolDefinition) -> Self {
        Self {
            r#type: "function".into(),
            function: FunctionDefinition {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
                strict: None,
            },
        }
    }
}

impl ToolDefinition {
    /// Apply strict mode to this tool definition.
    /// This sets `strict: true` and sanitizes the schema to meet OpenAI requirements.
    pub fn with_strict(mut self) -> Self {
        self.function.strict = Some(true);
        super::sanitize_schema(&mut self.function.parameters);
        self
    }
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    #[default]
    Auto,
    None,
    Required,
}

impl TryFrom<crate::message::ToolChoice> for ToolChoice {
    type Error = CompletionError;
    fn try_from(value: crate::message::ToolChoice) -> Result<Self, Self::Error> {
        let res = match value {
            message::ToolChoice::Specific { .. } => {
                return Err(CompletionError::ProviderError(
                    "Provider doesn't support only using specific tools".to_string(),
                ));
            }
            message::ToolChoice::Auto => Self::Auto,
            message::ToolChoice::None => Self::None,
            message::ToolChoice::Required => Self::Required,
        };

        Ok(res)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Function {
    pub name: String,
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
}

impl TryFrom<message::ToolResult> for Message {
    type Error = message::MessageError;

    fn try_from(value: message::ToolResult) -> Result<Self, Self::Error> {
        let text = value
            .content
            .into_iter()
            .map(|content| match content {
                message::ToolResultContent::Text(message::Text { text }) => Ok(text),
                _ => Err(message::MessageError::ConversionError(
                    "Tool result content does not support non-text".into(),
                )),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");

        Ok(Message::ToolResult {
            tool_call_id: value.id,
            content: ToolResultContentValue::String(text),
        })
    }
}

/// PROV-084: convert a rig `message::ToolResult` into one or more OpenAI
/// chat-completion `Message`s.
///
/// The OpenAI Chat Completions API only accepts string content on `tool`
/// role messages, so any image parts inside a tool result must be delivered
/// via a follow-up `user` role message immediately after the `tool`
/// message. Text parts are joined by newlines and placed on the tool
/// message; if the tool result is purely image parts, a non-empty
/// placeholder string is used so the server has something to render.
///
/// Returns:
/// - `[tool_msg]` for text-only tool results (preserves prior behaviour).
/// - `[tool_msg, user_msg]` for tool results containing one or more images.
fn tool_result_to_messages(
    value: message::ToolResult,
) -> Result<Vec<Message>, message::MessageError> {
    let tool_call_id = value.id;
    let mut text_parts: Vec<String> = Vec::new();
    let mut image_parts: Vec<UserContent> = Vec::new();

    for content in value.content.into_iter() {
        match content {
            message::ToolResultContent::Text(message::Text { text }) => {
                text_parts.push(text);
            }
            message::ToolResultContent::Image(message::Image {
                data,
                media_type,
                detail,
                ..
            }) => {
                let image_user_content = match data {
                    DocumentSourceKind::Url(url) => UserContent::Image {
                        image_url: ImageUrl {
                            url,
                            detail: detail.unwrap_or_default(),
                        },
                    },
                    DocumentSourceKind::Base64(b64) => {
                        let mime = media_type.map(|m| m.to_mime_type()).ok_or(
                            message::MessageError::ConversionError(
                                "OpenAI Image URI must have media type".into(),
                            ),
                        )?;
                        let url = format!("data:{mime};base64,{b64}");
                        UserContent::Image {
                            image_url: ImageUrl {
                                url,
                                detail: detail.unwrap_or_default(),
                            },
                        }
                    }
                    DocumentSourceKind::Raw(_) => {
                        return Err(message::MessageError::ConversionError(
                            "Raw tool-result images not supported, encode as base64 first".into(),
                        ));
                    }
                    DocumentSourceKind::Unknown => {
                        return Err(message::MessageError::ConversionError(
                            "Tool-result image has no body".into(),
                        ));
                    }
                    other => {
                        return Err(message::MessageError::ConversionError(format!(
                            "Unsupported tool-result image source: {other:?}"
                        )));
                    }
                };
                image_parts.push(image_user_content);
            }
        }
    }

    if image_parts.is_empty() {
        // Text-only tool result — match the legacy single-Message behaviour.
        Ok(vec![Message::ToolResult {
            tool_call_id,
            content: ToolResultContentValue::String(text_parts.join("\n")),
        }])
    } else {
        // Mixed / image-only tool result: emit a tool message carrying the
        // text (or a non-empty placeholder) followed by a user message
        // whose content array carries every image part.
        let tool_text = if text_parts.is_empty() {
            "[image attached below]".to_string()
        } else {
            text_parts.join("\n")
        };

        let tool_msg = Message::ToolResult {
            tool_call_id,
            content: ToolResultContentValue::String(tool_text),
        };

        let user_content = OneOrMany::many(image_parts).expect(
            "image_parts is guaranteed non-empty because we entered the image branch",
        );
        let user_msg = Message::User {
            content: user_content,
            name: None,
        };

        Ok(vec![tool_msg, user_msg])
    }
}

impl TryFrom<message::UserContent> for UserContent {
    type Error = message::MessageError;

    fn try_from(value: message::UserContent) -> Result<Self, Self::Error> {
        match value {
            message::UserContent::Text(message::Text { text }) => Ok(UserContent::Text { text }),
            message::UserContent::Image(message::Image {
                data,
                detail,
                media_type,
                ..
            }) => match data {
                DocumentSourceKind::Url(url) => Ok(UserContent::Image {
                    image_url: ImageUrl {
                        url,
                        detail: detail.unwrap_or_default(),
                    },
                }),
                DocumentSourceKind::Base64(data) => {
                    let url = format!(
                        "data:{};base64,{}",
                        media_type.map(|i| i.to_mime_type()).ok_or(
                            message::MessageError::ConversionError(
                                "OpenAI Image URI must have media type".into()
                            )
                        )?,
                        data
                    );

                    let detail = detail.unwrap_or_default();

                    Ok(UserContent::Image {
                        image_url: ImageUrl { url, detail },
                    })
                }
                DocumentSourceKind::Raw(_) => Err(message::MessageError::ConversionError(
                    "Raw files not supported, encode as base64 first".into(),
                )),
                DocumentSourceKind::Unknown => Err(message::MessageError::ConversionError(
                    "Document has no body".into(),
                )),
                doc => Err(message::MessageError::ConversionError(format!(
                    "Unsupported document type: {doc:?}"
                ))),
            },
            message::UserContent::Document(message::Document { data, .. }) => {
                if let DocumentSourceKind::Base64(text) | DocumentSourceKind::String(text) = data {
                    Ok(UserContent::Text { text })
                } else {
                    Err(message::MessageError::ConversionError(
                        "Documents must be base64 or a string".into(),
                    ))
                }
            }
            message::UserContent::Audio(message::Audio {
                data, media_type, ..
            }) => match data {
                DocumentSourceKind::Base64(data) => Ok(UserContent::Audio {
                    input_audio: InputAudio {
                        data,
                        format: match media_type {
                            Some(media_type) => media_type,
                            None => AudioMediaType::MP3,
                        },
                    },
                }),
                DocumentSourceKind::Url(_) => Err(message::MessageError::ConversionError(
                    "URLs are not supported for audio".into(),
                )),
                DocumentSourceKind::Raw(_) => Err(message::MessageError::ConversionError(
                    "Raw files are not supported for audio".into(),
                )),
                DocumentSourceKind::Unknown => Err(message::MessageError::ConversionError(
                    "Audio has no body".into(),
                )),
                audio => Err(message::MessageError::ConversionError(format!(
                    "Unsupported audio type: {audio:?}"
                ))),
            },
            message::UserContent::ToolResult(_) => Err(message::MessageError::ConversionError(
                "Tool result is in unsupported format".into(),
            )),
            message::UserContent::Video(_) => Err(message::MessageError::ConversionError(
                "Video is in unsupported format".into(),
            )),
        }
    }
}

impl TryFrom<OneOrMany<message::UserContent>> for Vec<Message> {
    type Error = message::MessageError;

    fn try_from(value: OneOrMany<message::UserContent>) -> Result<Self, Self::Error> {
        let (tool_results, other_content): (Vec<_>, Vec<_>) = value
            .into_iter()
            .partition(|content| matches!(content, message::UserContent::ToolResult(_)));

        // If there are messages with both tool results and user content, openai will only
        //  handle tool results. It's unlikely that there will be both.
        if !tool_results.is_empty() {
            // PROV-084: route every tool-result UserContent through
            // `tool_result_to_messages`, which emits a `tool` message plus
            // (if any image parts are present) a follow-up `user` message
            // carrying the `image_url` content parts. Flatten the per-
            // tool-result `Vec<Message>` into the final `Vec<Message>`.
            let mut out: Vec<Message> = Vec::new();
            for content in tool_results {
                let tool_result = match content {
                    message::UserContent::ToolResult(tr) => tr,
                    _ => unreachable!(
                        "partition above guarantees every element is UserContent::ToolResult"
                    ),
                };
                out.extend(tool_result_to_messages(tool_result)?);
            }
            Ok(out)
        } else {
            let other_content: Vec<UserContent> = other_content
                .into_iter()
                .map(|content| content.try_into())
                .collect::<Result<Vec<_>, _>>()?;

            let other_content = OneOrMany::many(other_content)
                .expect("There must be other content here if there were no tool result content");

            Ok(vec![Message::User {
                content: other_content,
                name: None,
            }])
        }
    }
}

impl TryFrom<OneOrMany<message::AssistantContent>> for Vec<Message> {
    type Error = message::MessageError;

    fn try_from(value: OneOrMany<message::AssistantContent>) -> Result<Self, Self::Error> {
        let (text_content, tool_calls) = value.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut texts, mut tools), content| {
                match content {
                    message::AssistantContent::Text(text) => texts.push(text),
                    message::AssistantContent::ToolCall(tool_call) => tools.push(tool_call),
                    // PROV-081: reasoning is INBOUND-only. Some providers
                    // (e.g. Fireworks hosting Qwen3) always emit
                    // `reasoning_content` on every assistant turn, which the
                    // inbound parser captures as `AssistantContent::Reasoning`
                    // and persists into chat history. On subsequent turns that
                    // history is converted back outbound via this path — we
                    // must silently drop reasoning here rather than panic,
                    // otherwise any multi-turn conversation against such a
                    // provider crashes on turn 2. The explicit `reasoning:
                    // None` below plus `skip_serializing_if` on
                    // `Message::Assistant::reasoning` still guarantees the
                    // outbound wire never carries a `reasoning` /
                    // `reasoning_content` key.
                    message::AssistantContent::Reasoning(_) => {}
                    // Images in assistant messages are likewise not supported
                    // by the OpenAI Completions wire format. Drop rather than
                    // panic so a stray inbound image block can't kill an
                    // otherwise-healthy conversation.
                    message::AssistantContent::Image(_) => {}
                }
                (texts, tools)
            },
        );

        // `OneOrMany` ensures at least one `AssistantContent::Text` or `ToolCall` exists,
        //  so either `content` or `tool_calls` will have some content.
        Ok(vec![Message::Assistant {
            content: text_content
                .into_iter()
                .map(|content| content.text.into())
                .collect::<Vec<_>>(),
            refusal: None,
            audio: None,
            name: None,
            // PROV-081: reasoning is an INBOUND-only field. We never plumb
            // captured reasoning back out to the server — the panic at
            // `message::AssistantContent::Reasoning` above is the last line of
            // defense; this explicit `None` plus the `skip_serializing_if` on
            // `Message::Assistant::reasoning` ensures the outbound wire never
            // carries a `reasoning` / `reasoning_content` key.
            reasoning: None,
            tool_calls: tool_calls
                .into_iter()
                .map(|tool_call| tool_call.into())
                .collect::<Vec<_>>(),
        }])
    }
}

impl TryFrom<message::Message> for Vec<Message> {
    type Error = message::MessageError;

    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        match message {
            message::Message::User { content } => content.try_into(),
            message::Message::Assistant { content, .. } => content.try_into(),
        }
    }
}

impl From<message::ToolCall> for ToolCall {
    fn from(tool_call: message::ToolCall) -> Self {
        Self {
            id: tool_call.id,
            r#type: ToolType::default(),
            function: Function {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}

impl From<ToolCall> for message::ToolCall {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            id: tool_call.id,
            call_id: None,
            function: message::ToolFunction {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
            signature: None,
            additional_params: None,
        }
    }
}

impl TryFrom<Message> for message::Message {
    type Error = message::MessageError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        Ok(match message {
            Message::User { content, .. } => message::Message::User {
                content: content.map(|content| content.into()),
            },
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                let mut content = content
                    .into_iter()
                    .map(|content| match content {
                        AssistantContent::Text { text } => message::AssistantContent::text(text),

                        // TODO: Currently, refusals are converted into text, but should be
                        //  investigated for generalization.
                        AssistantContent::Refusal { refusal } => {
                            message::AssistantContent::text(refusal)
                        }
                    })
                    .collect::<Vec<_>>();

                content.extend(
                    tool_calls
                        .into_iter()
                        .map(|tool_call| Ok(message::AssistantContent::ToolCall(tool_call.into())))
                        .collect::<Result<Vec<_>, _>>()?,
                );

                message::Message::Assistant {
                    id: None,
                    content: OneOrMany::many(content).map_err(|_| {
                        message::MessageError::ConversionError(
                            "Neither `content` nor `tool_calls` was provided to the Message"
                                .to_owned(),
                        )
                    })?,
                }
            }

            Message::ToolResult {
                tool_call_id,
                content,
            } => message::Message::User {
                content: OneOrMany::one(message::UserContent::tool_result(
                    tool_call_id,
                    OneOrMany::one(message::ToolResultContent::text(content.as_text())),
                )),
            },

            // System messages should get stripped out when converting messages, this is just a
            // stop gap to avoid obnoxious error handling or panic occurring.
            Message::System { content, .. } => message::Message::User {
                content: content.map(|content| message::UserContent::text(content.text)),
            },
        })
    }
}

impl From<UserContent> for message::UserContent {
    fn from(content: UserContent) -> Self {
        match content {
            UserContent::Text { text } => message::UserContent::text(text),
            UserContent::Image { image_url } => {
                message::UserContent::image_url(image_url.url, None, Some(image_url.detail))
            }
            UserContent::Audio { input_audio } => {
                message::UserContent::audio(input_audio.data, Some(input_audio.format))
            }
        }
    }
}

impl From<String> for UserContent {
    fn from(s: String) -> Self {
        UserContent::Text { text: s }
    }
}

impl FromStr for UserContent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserContent::Text {
            text: s.to_string(),
        })
    }
}

impl From<String> for AssistantContent {
    fn from(s: String) -> Self {
        AssistantContent::Text { text: s }
    }
}

impl FromStr for AssistantContent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(AssistantContent::Text {
            text: s.to_string(),
        })
    }
}
impl From<String> for SystemContent {
    fn from(s: String) -> Self {
        SystemContent {
            r#type: SystemContentType::default(),
            text: s,
        }
    }
}

impl FromStr for SystemContent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SystemContent {
            r#type: SystemContentType::default(),
            text: s.to_string(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub system_fingerprint: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        let content = match &choice.message {
            Message::Assistant {
                content,
                tool_calls,
                reasoning,
                ..
            } => {
                let mut out: Vec<completion::AssistantContent> = Vec::new();

                // PROV-081: surface reasoning/thinking text BEFORE text and
                // tool-call entries, so downstream consumers see the model's
                // analysis ahead of its final answer. Captures vLLM's
                // `reasoning` field and (via the serde alias) Z.AI/GLM's
                // `reasoning_content` field.
                if let Some(reasoning_text) = reasoning {
                    if !reasoning_text.is_empty() {
                        out.push(completion::AssistantContent::reasoning(reasoning_text));
                    }
                }

                out.extend(content.iter().filter_map(|c| {
                    let s = match c {
                        AssistantContent::Text { text } => text,
                        AssistantContent::Refusal { refusal } => refusal,
                    };
                    if s.is_empty() {
                        None
                    } else {
                        Some(completion::AssistantContent::text(s))
                    }
                }));

                out.extend(tool_calls.iter().map(|call| {
                    completion::AssistantContent::tool_call(
                        &call.id,
                        &call.function.name,
                        call.function.arguments.clone(),
                    )
                }));
                Ok(out)
            }
            _ => Err(CompletionError::ResponseError(
                "Response did not contain a valid message or tool call".into(),
            )),
        }?;

        let choice = OneOrMany::many(content).map_err(|_| {
            CompletionError::ResponseError(
                "Response contained no message or tool call (empty)".to_owned(),
            )
        })?;

        let usage = response
            .usage
            .as_ref()
            .map(|usage| {
                let mut u = completion::Usage {
                    input_tokens: usage.prompt_tokens as u64,
                    output_tokens: (usage.total_tokens - usage.prompt_tokens) as u64,
                    total_tokens: usage.total_tokens as u64,
                    ..Default::default()
                };
                if let Some(details) = &usage.prompt_tokens_details {
                    u.cache_read_input_tokens = Some(details.cached_tokens as u64);
                }
                if let Some(details) = &usage.completion_tokens_details {
                    u.reasoning_tokens = Some(details.reasoning_tokens as u64);
                }
                u
            })
            .unwrap_or_default();

        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }
}

impl ProviderResponseExt for CompletionResponse {
    type OutputMessage = Choice;
    type Usage = Usage;

    fn get_response_id(&self) -> Option<String> {
        Some(self.id.to_owned())
    }

    fn get_response_model_name(&self) -> Option<String> {
        Some(self.model.to_owned())
    }

    fn get_output_messages(&self) -> Vec<Self::OutputMessage> {
        self.choices.clone()
    }

    fn get_text_response(&self) -> Option<String> {
        let Message::User { ref content, .. } = self.choices.last()?.message.clone() else {
            return None;
        };

        let UserContent::Text { text } = content.first() else {
            return None;
        };

        Some(text)
    }

    fn get_usage(&self) -> Option<Self::Usage> {
        self.usage.clone()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: String,
}

/// Details about prompt tokens, including cached tokens
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: usize,
}

/// Details about completion tokens, including reasoning tokens
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    /// Completion (output) tokens - directly provided by API
    #[serde(default)]
    pub completion_tokens: Option<usize>,
    pub total_tokens: usize,
    /// Details about prompt tokens (Z.AI/OpenAI caching)
    #[serde(default)]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Details about completion tokens (reasoning tokens)
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

impl Usage {
    pub fn new() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: None,
            total_tokens: 0,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }
    }

    /// Calculate output tokens, using completion_tokens if available, otherwise deriving from total - prompt
    pub fn output_tokens(&self) -> u64 {
        self.completion_tokens
            .map(|c| c as u64)
            .unwrap_or_else(|| (self.total_tokens - self.prompt_tokens) as u64)
    }
}

impl Default for Usage {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cached = self.prompt_tokens_details.as_ref().map(|d| d.cached_tokens).unwrap_or(0);
        write!(
            f,
            "Prompt tokens: {} (cached: {}) Completion tokens: {} Total tokens: {}",
            self.prompt_tokens, cached, self.output_tokens(), self.total_tokens
        )
    }
}

impl GetTokenUsage for Usage {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();
        usage.input_tokens = self.prompt_tokens as u64;
        usage.output_tokens = self.output_tokens();
        usage.total_tokens = self.total_tokens as u64;
        // Z.AI/OpenAI cache tokens
        if let Some(details) = &self.prompt_tokens_details {
            usage.cache_read_input_tokens = Some(details.cached_tokens as u64);
        }
        // Reasoning tokens
        if let Some(details) = &self.completion_tokens_details {
            usage.reasoning_tokens = Some(details.reasoning_tokens as u64);
        }
        Some(usage)
    }
}

#[derive(Clone)]
pub struct CompletionModel<T = reqwest::Client> {
    pub(crate) client: Client<T>,
    pub model: String,
    pub strict_tools: bool,
    pub tool_result_array_content: bool,
}

impl<T> CompletionModel<T>
where
    T: Default + std::fmt::Debug + Clone + 'static,
{
    pub fn new(client: Client<T>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            strict_tools: false,
            tool_result_array_content: false,
        }
    }

    pub fn with_model(client: Client<T>, model: &str) -> Self {
        Self {
            client,
            model: model.into(),
            strict_tools: false,
            tool_result_array_content: false,
        }
    }

    /// Enable strict mode for tool schemas.
    ///
    /// When enabled, tool schemas are automatically sanitized to meet OpenAI's strict mode requirements:
    /// - `additionalProperties: false` is added to all objects
    /// - All properties are marked as required
    /// - `strict: true` is set on each function definition
    ///
    /// This allows OpenAI to guarantee that the model's tool calls will match the schema exactly.
    pub fn with_strict_tools(mut self) -> Self {
        self.strict_tools = true;
        self
    }

    pub fn with_tool_result_array_content(mut self) -> Self {
        self.tool_result_array_content = true;
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(flatten)]
    additional_params: Option<serde_json::Value>,
}

/// Parameters used to build an OpenAI Chat Completions `CompletionRequest`.
///
/// **PROV-081 passthrough caveat:** `request.additional_params` is serialized
/// flat into the outgoing JSON body via `#[serde(flatten)]` on
/// `CompletionRequest::additional_params`. Callers MUST NOT set either
/// `include_reasoning: false` or `chat_template_kwargs.enable_thinking: false`
/// via `additional_params` — those keys suppress server-side reasoning on
/// vLLM / Qwen3 and would silently defeat the PROV-081 reasoning-capture path.
/// The default-built request never emits these keys (see
/// `prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys`).
pub struct OpenAIRequestParams {
    pub model: String,
    pub request: CoreCompletionRequest,
    pub strict_tools: bool,
    pub tool_result_array_content: bool,
}

impl TryFrom<OpenAIRequestParams> for CompletionRequest {
    type Error = CompletionError;

    fn try_from(params: OpenAIRequestParams) -> Result<Self, Self::Error> {
        let OpenAIRequestParams {
            model,
            request: req,
            strict_tools,
            tool_result_array_content,
        } = params;

        let mut partial_history = vec![];
        if let Some(docs) = req.normalized_documents() {
            partial_history.push(docs);
        }
        let CoreCompletionRequest {
            preamble,
            chat_history,
            tools,
            temperature,
            additional_params,
            tool_choice,
            ..
        } = req;

        partial_history.extend(chat_history);

        let mut full_history: Vec<Message> =
            preamble.map_or_else(Vec::new, |preamble| vec![Message::system(&preamble)]);

        full_history.extend(
            partial_history
                .into_iter()
                .map(message::Message::try_into)
                .collect::<Result<Vec<Vec<Message>>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        );

        if tool_result_array_content {
            for msg in &mut full_history {
                if let Message::ToolResult { content, .. } = msg {
                    *content = content.to_array();
                }
            }
        }

        let tool_choice = tool_choice.map(ToolChoice::try_from).transpose()?;

        let tools: Vec<ToolDefinition> = tools
            .into_iter()
            .map(|tool| {
                let def = ToolDefinition::from(tool);
                if strict_tools { def.with_strict() } else { def }
            })
            .collect();

        let res = Self {
            model,
            messages: full_history,
            tools,
            tool_choice,
            temperature,
            additional_params,
        };

        Ok(res)
    }
}

impl TryFrom<(String, CoreCompletionRequest)> for CompletionRequest {
    type Error = CompletionError;

    fn try_from((model, req): (String, CoreCompletionRequest)) -> Result<Self, Self::Error> {
        CompletionRequest::try_from(OpenAIRequestParams {
            model,
            request: req,
            strict_tools: false,
            tool_result_array_content: false,
        })
    }
}

impl crate::telemetry::ProviderRequestExt for CompletionRequest {
    type InputMessage = Message;

    fn get_input_messages(&self) -> Vec<Self::InputMessage> {
        self.messages.clone()
    }

    fn get_system_prompt(&self) -> Option<String> {
        let first_message = self.messages.first()?;

        let Message::System { ref content, .. } = first_message.clone() else {
            return None;
        };

        let SystemContent { text, .. } = content.first();

        Some(text)
    }

    fn get_prompt(&self) -> Option<String> {
        let last_message = self.messages.last()?;

        let Message::User { ref content, .. } = last_message.clone() else {
            return None;
        };

        let UserContent::Text { text } = content.first() else {
            return None;
        };

        Some(text)
    }

    fn get_model_name(&self) -> String {
        self.model.clone()
    }
}

impl CompletionModel<reqwest::Client> {
    pub fn into_agent_builder(self) -> crate::agent::AgentBuilder<Self> {
        crate::agent::AgentBuilder::new(self)
    }
}

impl<T> completion::CompletionModel for CompletionModel<T>
where
    T: HttpClientExt
        + Default
        + std::fmt::Debug
        + Clone
        + WasmCompatSend
        + WasmCompatSync
        + 'static,
{
    type Response = CompletionResponse;
    type StreamingResponse = StreamingCompletionResponse;

    type Client = super::CompletionsClient<T>;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model)
    }

    async fn completion(
        &self,
        completion_request: CoreCompletionRequest,
    ) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "openai",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = &completion_request.preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        let request = CompletionRequest::try_from(OpenAIRequestParams {
            model: self.model.to_owned(),
            request: completion_request,
            strict_tools: self.strict_tools,
            tool_result_array_content: self.tool_result_array_content,
        })?;

        if enabled!(Level::TRACE) {
            tracing::trace!(
                target: "rig::completions",
                "OpenAI Chat Completions completion request: {}",
                serde_json::to_string_pretty(&request)?
            );
        }

        let body = serde_json::to_vec(&request)?;

        let req = self
            .client
            .post("/chat/completions")?
            .body(body)
            .map_err(|e| CompletionError::HttpError(e.into()))?;

        async move {
            let response = self.client.send(req).await?;

            if response.status().is_success() {
                let text = http_client::text(response).await?;

                match serde_json::from_str::<ApiResponse<CompletionResponse>>(&text)? {
                    ApiResponse::Ok(response) => {
                        let span = tracing::Span::current();
                        span.record_response_metadata(&response);
                        span.record_token_usage(&response.usage);

                        if enabled!(Level::TRACE) {
                            tracing::trace!(
                                target: "rig::completions",
                                "OpenAI Chat Completions completion response: {}",
                                serde_json::to_string_pretty(&response)?
                            );
                        }

                        response.try_into()
                    }
                    ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
                }
            } else {
                let text = http_client::text(response).await?;
                Err(CompletionError::ProviderError(text))
            }
        }
        .instrument(span)
        .await
    }

    async fn stream(
        &self,
        request: CoreCompletionRequest,
    ) -> Result<
        crate::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        CompletionError,
    > {
        Self::stream(self, request).await
    }
}

// ================================================================
// PROV-081: vLLM-native `reasoning` field capture (non-streaming)
//
// Feature: spec/features/openai-provider-reasoning-tokens.feature
//
// These tests validate that the OpenAI provider recognizes the
// `reasoning` field emitted by vLLM (Qwen3 reasoning-parser output)
// in addition to the Z.AI/GLM `reasoning_content` field on
// non-streaming assistant messages, AND that the outgoing request
// body never emits reasoning-suppression keys.
// ================================================================
#[cfg(test)]
mod prov_081_tests {
    use super::*;
    use crate::OneOrMany;
    use crate::completion;
    use crate::streaming as core_streaming;

    // ------------------------------------------------------------
    // Streaming helpers (reused across the four streaming scenarios)
    // ------------------------------------------------------------

    /// Mock HTTP client that serves a canned SSE payload to the streaming decoder.
    #[derive(Clone)]
    struct MockSseClient {
        sse_bytes: bytes::Bytes,
    }

    impl crate::http_client::HttpClientExt for MockSseClient {
        fn send<T, U>(
            &self,
            _req: http::Request<T>,
        ) -> impl std::future::Future<
            Output = crate::http_client::Result<
                http::Response<crate::http_client::LazyBody<U>>,
            >,
        > + crate::wasm_compat::WasmCompatSend
        + 'static
        where
            T: Into<bytes::Bytes>,
            T: crate::wasm_compat::WasmCompatSend,
            U: From<bytes::Bytes>,
            U: crate::wasm_compat::WasmCompatSend + 'static,
        {
            std::future::ready(Err(crate::http_client::Error::InvalidStatusCode(
                http::StatusCode::NOT_IMPLEMENTED,
            )))
        }

        fn send_multipart<U>(
            &self,
            _req: http::Request<crate::http_client::MultipartForm>,
        ) -> impl std::future::Future<
            Output = crate::http_client::Result<
                http::Response<crate::http_client::LazyBody<U>>,
            >,
        > + crate::wasm_compat::WasmCompatSend
        + 'static
        where
            U: From<bytes::Bytes>,
            U: crate::wasm_compat::WasmCompatSend + 'static,
        {
            std::future::ready(Err(crate::http_client::Error::InvalidStatusCode(
                http::StatusCode::NOT_IMPLEMENTED,
            )))
        }

        fn send_streaming<T>(
            &self,
            _req: http::Request<T>,
        ) -> impl std::future::Future<
            Output = crate::http_client::Result<crate::http_client::StreamingResponse>,
        > + crate::wasm_compat::WasmCompatSend
        where
            T: Into<bytes::Bytes>,
        {
            let sse_bytes = self.sse_bytes.clone();
            async move {
                let byte_stream = futures::stream::iter(vec![Ok::<
                    bytes::Bytes,
                    crate::http_client::Error,
                >(sse_bytes)]);
                let boxed_stream: crate::http_client::sse::BoxedStream = Box::pin(byte_stream);

                http::Response::builder()
                    .status(http::StatusCode::OK)
                    .header(reqwest::header::CONTENT_TYPE, "text/event-stream")
                    .body(boxed_stream)
                    .map_err(crate::http_client::Error::Protocol)
            }
        }
    }

    /// Drive an SSE payload through `send_compatible_streaming_request` and collect
    /// every emitted `StreamedAssistantContent` variant for assertions.
    async fn collect_streamed_contents(
        sse: &'static str,
    ) -> Vec<core_streaming::StreamedAssistantContent<streaming::StreamingCompletionResponse>>
    {
        use futures::StreamExt;

        let client = MockSseClient {
            sse_bytes: bytes::Bytes::from(sse),
        };

        let req = http::Request::builder()
            .method("POST")
            .uri("http://localhost/v1/chat/completions")
            .body(Vec::new())
            .expect("build mock request");

        let mut stream = streaming::send_compatible_streaming_request(client, req)
            .await
            .expect("stream init should succeed");

        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            let value = chunk.expect("each chunk should be Ok");
            let is_final =
                matches!(value, core_streaming::StreamedAssistantContent::Final(_));
            out.push(value);
            if is_final {
                break;
            }
        }
        out
    }

    /// Build the JSON body that would be POSTed to `/chat/completions`
    /// for a plain caller request (mirrors what `completion::CompletionModel::completion`
    /// does internally, minus the HTTP send).
    fn build_request_body_json() -> serde_json::Value {
        // A minimal caller-built CompletionRequest (no reasoning-suppression flags).
        let core_req = crate::completion::CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(crate::completion::Message::User {
                content: OneOrMany::one(crate::message::UserContent::text("hi")),
            }),
            documents: Vec::new(),
            tools: Vec::new(),
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let wire_req = CompletionRequest::try_from(OpenAIRequestParams {
            model: "qwen".to_string(),
            request: core_req,
            strict_tools: false,
            tool_result_array_content: false,
        })
        .expect("building request should succeed");

        serde_json::to_value(&wire_req).expect("serializing request should succeed")
    }

    /// Extract reasoning text from a CompletionResponse's choice (OneOrMany<AssistantContent>).
    fn extract_reasoning_texts(
        response: &completion::CompletionResponse<CompletionResponse>,
    ) -> Vec<String> {
        response
            .choice
            .iter()
            .filter_map(|c| match c {
                completion::AssistantContent::Reasoning(r) => Some(r.reasoning.join("")),
                _ => None,
            })
            .collect()
    }

    /// Extract text content from a CompletionResponse's choice.
    fn extract_text_contents(
        response: &completion::CompletionResponse<CompletionResponse>,
    ) -> Vec<String> {
        response
            .choice
            .iter()
            .filter_map(|c| match c {
                completion::AssistantContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn prov_081_non_streaming_vllm_reasoning_field_surfaces_reasoning_and_content() {
        // @step Given a caller issues a non-streaming chat completion through the OpenAI provider
        // @step And the upstream server returns a response whose assistant message is {"role":"assistant","reasoning":"analysis","content":"answer"}
        let body = serde_json::json!({
            "id": "resp-1",
            "object": "chat.completion",
            "created": 1,
            "model": "qwen",
            "system_fingerprint": null,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "reasoning": "analysis",
                    "content": "answer"
                },
                "logprobs": null,
                "finish_reason": "stop"
            }],
            "usage": null
        });

        let raw: CompletionResponse =
            serde_json::from_value(body).expect("deserialize raw provider response");

        // @step When the provider decodes the response
        let response: completion::CompletionResponse<CompletionResponse> =
            raw.try_into().expect("convert into caller-visible response");

        // @step Then the caller-visible CompletionResponse exposes reasoning text "analysis"
        let reasoning = extract_reasoning_texts(&response);
        assert_eq!(
            reasoning,
            vec!["analysis".to_string()],
            "expected reasoning text 'analysis' surfaced from vLLM `reasoning` field"
        );

        // @step And the caller-visible CompletionResponse exposes content text "answer"
        let content = extract_text_contents(&response);
        assert_eq!(
            content,
            vec!["answer".to_string()],
            "expected content text 'answer' surfaced from `content` field"
        );
    }

    #[test]
    fn prov_081_non_streaming_glm_reasoning_content_field_surfaces_reasoning_and_content() {
        // @step Given a caller issues a non-streaming chat completion through the OpenAI provider
        // @step And the upstream server returns a response whose assistant message is {"role":"assistant","reasoning_content":"analysis","content":"answer"}
        let body = serde_json::json!({
            "id": "resp-2",
            "object": "chat.completion",
            "created": 1,
            "model": "glm-4",
            "system_fingerprint": null,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "reasoning_content": "analysis",
                    "content": "answer"
                },
                "logprobs": null,
                "finish_reason": "stop"
            }],
            "usage": null
        });

        let raw: CompletionResponse =
            serde_json::from_value(body).expect("deserialize raw provider response");

        // @step When the provider decodes the response
        let response: completion::CompletionResponse<CompletionResponse> =
            raw.try_into().expect("convert into caller-visible response");

        // @step Then the caller-visible CompletionResponse exposes reasoning text "analysis"
        let reasoning = extract_reasoning_texts(&response);
        assert_eq!(
            reasoning,
            vec!["analysis".to_string()],
            "expected reasoning text 'analysis' surfaced from GLM `reasoning_content` field"
        );

        // @step And the caller-visible CompletionResponse exposes content text "answer"
        let content = extract_text_contents(&response);
        assert_eq!(content, vec!["answer".to_string()]);
    }

    #[test]
    fn prov_081_non_streaming_without_reasoning_field_still_works() {
        // @step Given a caller issues a non-streaming chat completion through the OpenAI provider
        // @step And the upstream server returns a response whose assistant message is {"role":"assistant","content":"answer"}
        let body = serde_json::json!({
            "id": "resp-3",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o",
            "system_fingerprint": null,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "answer"
                },
                "logprobs": null,
                "finish_reason": "stop"
            }],
            "usage": null
        });

        // @step When the provider decodes the response
        let raw: CompletionResponse = serde_json::from_value(body)
            .expect("plain response must still deserialize with no reasoning field");
        let response: completion::CompletionResponse<CompletionResponse> =
            raw.try_into().expect("convert into caller-visible response");

        // @step Then the caller-visible CompletionResponse exposes content text "answer"
        let content = extract_text_contents(&response);
        assert_eq!(content, vec!["answer".to_string()]);

        // @step And the caller-visible CompletionResponse does not expose any reasoning text
        let reasoning = extract_reasoning_texts(&response);
        assert!(
            reasoning.is_empty(),
            "expected no reasoning text surfaced, got {reasoning:?}"
        );

        // @step And the response does not produce a decode error
        // (deserialize + try_into above both unwrapped — any error would have panicked)
    }

    #[test]
    fn prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys() {
        // @step Given a caller builds a chat completion request through the OpenAI provider without explicitly setting any reasoning-suppression flag
        // @step When the provider serializes the request body that would be POSTed to /chat/completions
        let body = build_request_body_json();

        // @step Then the serialized body does not contain a top-level key named "include_reasoning"
        let obj = body
            .as_object()
            .expect("serialized body should be a JSON object");
        assert!(
            !obj.contains_key("include_reasoning"),
            "request body must NOT carry `include_reasoning` (vLLM strips reasoning when false). Body: {body}"
        );

        // @step And the serialized body does not contain a "chat_template_kwargs.enable_thinking" key path
        if let Some(chat_template_kwargs) = obj.get("chat_template_kwargs") {
            if let Some(ctk) = chat_template_kwargs.as_object() {
                assert!(
                    !ctk.contains_key("enable_thinking"),
                    "request body must NOT carry `chat_template_kwargs.enable_thinking` (Qwen3 template-level kill-switch). Body: {body}"
                );
            }
        }
    }

    // ------------------------------------------------------------
    // Scenario 1: Streaming — vLLM-native `reasoning` field
    // ------------------------------------------------------------
    #[tokio::test]
    async fn prov_081_streaming_vllm_reasoning_field_surfaces_as_reasoning_delta() {
        // @step Given a caller streams a chat completion through the OpenAI provider
        // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning":"Let me analyse..."}}]}
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning\":\"Let me analyse...\"}}]}\n\n",
            "data: [DONE]\n\n",
        );

        // @step When the provider decodes the chunk
        let events = collect_streamed_contents(sse).await;

        // @step Then the caller receives a ReasoningDelta whose reasoning text equals "Let me analyse..."
        let reasoning_texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                core_streaming::StreamedAssistantContent::ReasoningDelta {
                    reasoning, ..
                } => Some(reasoning.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            reasoning_texts,
            vec!["Let me analyse...".to_string()],
            "expected exactly one ReasoningDelta carrying the vLLM `reasoning` field content"
        );

        // @step And the caller does not receive any content delta for that chunk
        for e in &events {
            if let core_streaming::StreamedAssistantContent::Text(t) = e {
                panic!("unexpected content delta: {:?}", t);
            }
        }

        // @step And the chunk does not produce a decode error
        // (collect_streamed_contents unwraps each chunk — a decode error would have panicked above)
    }

    // ------------------------------------------------------------
    // Scenario 2: Streaming — Z.AI/GLM `reasoning_content` field
    // ------------------------------------------------------------
    #[tokio::test]
    async fn prov_081_streaming_glm_reasoning_content_field_surfaces_as_reasoning_delta() {
        // @step Given a caller streams a chat completion through the OpenAI provider
        // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking...\"}}]}\n\n",
            "data: [DONE]\n\n",
        );

        // @step When the provider decodes the chunk
        let events = collect_streamed_contents(sse).await;

        // @step Then the caller receives a ReasoningDelta whose reasoning text equals "thinking..."
        let reasoning_texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                core_streaming::StreamedAssistantContent::ReasoningDelta {
                    reasoning, ..
                } => Some(reasoning.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            reasoning_texts,
            vec!["thinking...".to_string()],
            "expected exactly one ReasoningDelta carrying the GLM `reasoning_content` field content"
        );

        // @step And the caller does not receive any content delta for that chunk
        for e in &events {
            if let core_streaming::StreamedAssistantContent::Text(t) = e {
                panic!("unexpected content delta: {:?}", t);
            }
        }
    }

    // ------------------------------------------------------------
    // Scenario 3: Streaming — concatenate when both fields present
    // ------------------------------------------------------------
    #[tokio::test]
    async fn prov_081_streaming_concatenates_reasoning_when_both_fields_present() {
        // @step Given a caller streams a chat completion through the OpenAI provider
        // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning_content":"B","reasoning":"A"}}]}
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"B\",\"reasoning\":\"A\"}}]}\n\n",
            "data: [DONE]\n\n",
        );

        // @step When the provider decodes the chunk
        let events = collect_streamed_contents(sse).await;

        // @step Then the caller receives reasoning text equal to "BA" with reasoning_content concatenated first and reasoning appended
        let combined: String = events
            .iter()
            .filter_map(|e| match e {
                core_streaming::StreamedAssistantContent::ReasoningDelta {
                    reasoning, ..
                } => Some(reasoning.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert_eq!(
            combined, "BA",
            "expected reasoning output to be exactly 'BA' \
             (reasoning_content='B' concatenated first, reasoning='A' appended), \
             got {combined:?}"
        );

        // @step And no reasoning character from either source field is dropped
        assert_eq!(
            combined.chars().filter(|c| *c == 'A').count(),
            1,
            "character 'A' must appear exactly once (not dropped, not duplicated)"
        );
        assert_eq!(
            combined.chars().filter(|c| *c == 'B').count(),
            1,
            "character 'B' must appear exactly once (not dropped, not duplicated)"
        );
    }

    // ------------------------------------------------------------
    // Scenario 4: Streaming — regression: plain content still works
    // ------------------------------------------------------------
    #[tokio::test]
    async fn prov_081_streaming_content_chunk_passes_through_unchanged() {
        // @step Given a caller streams a chat completion through the OpenAI provider
        // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"content":"hello"}}]}
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n",
            "data: [DONE]\n\n",
        );

        // @step When the provider decodes the chunk
        let events = collect_streamed_contents(sse).await;

        // @step Then the caller receives a content delta whose text equals "hello"
        let content_texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                core_streaming::StreamedAssistantContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(content_texts, vec!["hello".to_string()]);

        // @step And the caller does not receive any ReasoningDelta
        for e in &events {
            if let core_streaming::StreamedAssistantContent::ReasoningDelta {
                reasoning, ..
            } = e
            {
                panic!("unexpected ReasoningDelta: {:?}", reasoning);
            }
        }

        // @step And the chunk does not produce a decode error
        // (collect_streamed_contents unwraps each chunk — a decode error would have panicked above)
    }
}

// ================================================================
// PROV-083 — Base64 user images with detail=None must default to "auto"
//
// Feature: spec/features/openai-provider-base64-image-inputs.feature
//
// Covers the five scenarios that validate
// `impl TryFrom<message::UserContent> for UserContent` handles:
//   1. Base64 + PNG + detail=None         -> detail "auto"
//   2. Base64 + JPEG + detail=None        -> detail "auto"
//   3. Base64 + PNG + detail=Some(High)   -> detail "high"
//   4. URL + PNG + detail=None            -> unchanged behaviour, detail "auto"
//   5. Base64 + media_type=None           -> ConversionError about missing MIME
// ================================================================
#[cfg(test)]
mod prov_083_tests {
    //! Feature: spec/features/openai-provider-base64-image-inputs.feature
    //!
    //! Covers PROV-083 — base64 user images with detail=None must default to "auto".
    use super::*;
    use crate::message::{ImageMediaType, UserContent as RigUserContent};

    /// Helper: build a rig `message::UserContent::Image` with the requested data/media/detail.
    fn build_rig_image(
        data: DocumentSourceKind,
        media_type: Option<ImageMediaType>,
        detail: Option<ImageDetail>,
    ) -> RigUserContent {
        RigUserContent::Image(message::Image {
            data,
            media_type,
            detail,
            additional_params: None,
        })
    }

    /// Helper: run the provider conversion and return the wire `UserContent` on success.
    fn convert(content: RigUserContent) -> Result<UserContent, message::MessageError> {
        UserContent::try_from(content)
    }

    #[test]
    fn prov_083_base64_png_with_detail_none_defaults_to_auto() {
        // @step Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=image/png, and detail=None
        let rig_content = build_rig_image(
            DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg".into()),
            Some(ImageMediaType::PNG),
            None,
        );

        // @step When the provider converts the message into an OpenAI chat completion Message
        let result = convert(rig_content);

        // @step Then the conversion succeeds
        let wire = result.expect("base64 image with detail=None should convert successfully");

        // @step And the resulting user content part has type "image_url"
        // @step And the resulting image_url.url starts with "data:image/png;base64,"
        // @step And the resulting image_url.detail equals "auto"
        match wire {
            UserContent::Image { image_url } => {
                assert!(
                    image_url.url.starts_with("data:image/png;base64,"),
                    "unexpected url prefix: {}",
                    image_url.url
                );
                assert_eq!(image_url.detail, ImageDetail::Auto);

                // Also assert the serialized JSON shape carries the right type tag + detail string.
                let wire_json = serde_json::to_value(UserContent::Image { image_url })
                    .expect("serialize UserContent::Image");
                assert_eq!(wire_json["type"], "image_url");
                assert_eq!(wire_json["image_url"]["detail"], "auto");
                assert!(
                    wire_json["image_url"]["url"]
                        .as_str()
                        .expect("url is a string")
                        .starts_with("data:image/png;base64,")
                );
            }
            other => panic!("expected UserContent::Image, got {other:?}"),
        }
    }

    #[test]
    fn prov_083_base64_jpeg_with_detail_none_defaults_to_auto() {
        // @step Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=image/jpeg, and detail=None
        let rig_content = build_rig_image(
            DocumentSourceKind::Base64("/9j/4AAQSkZJRgABAQEAYABgAAD".into()),
            Some(ImageMediaType::JPEG),
            None,
        );

        // @step When the provider converts the message into an OpenAI chat completion Message
        let result = convert(rig_content);

        // @step Then the conversion succeeds
        let wire = result.expect("base64 JPEG with detail=None should convert successfully");

        // @step And the resulting image_url.url starts with "data:image/jpeg;base64,"
        // @step And the resulting image_url.detail equals "auto"
        match wire {
            UserContent::Image { image_url } => {
                assert!(
                    image_url.url.starts_with("data:image/jpeg;base64,"),
                    "unexpected url prefix: {}",
                    image_url.url
                );
                assert_eq!(image_url.detail, ImageDetail::Auto);
            }
            other => panic!("expected UserContent::Image, got {other:?}"),
        }
    }

    #[test]
    fn prov_083_base64_png_with_explicit_high_preserves_value() {
        // @step Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=image/png, and detail=High
        let rig_content = build_rig_image(
            DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg".into()),
            Some(ImageMediaType::PNG),
            Some(ImageDetail::High),
        );

        // @step When the provider converts the message into an OpenAI chat completion Message
        let result = convert(rig_content);

        // @step Then the conversion succeeds
        let wire = result.expect("base64 image with detail=High should convert successfully");

        // @step And the resulting image_url.detail equals "high"
        match wire {
            UserContent::Image { image_url } => {
                assert_eq!(image_url.detail, ImageDetail::High);
                let wire_json = serde_json::to_value(UserContent::Image { image_url })
                    .expect("serialize UserContent::Image");
                assert_eq!(wire_json["image_url"]["detail"], "high");
            }
            other => panic!("expected UserContent::Image, got {other:?}"),
        }
    }

    #[test]
    fn prov_083_url_image_path_unchanged() {
        // @step Given a caller builds a rig `message::UserContent::Image` with a https URL data source, media_type=image/png, and detail=None
        let original_url = "https://example.com/cat.png";
        let rig_content = build_rig_image(
            DocumentSourceKind::Url(original_url.into()),
            Some(ImageMediaType::PNG),
            None,
        );

        // @step When the provider converts the message into an OpenAI chat completion Message
        let result = convert(rig_content);

        // @step Then the conversion succeeds
        let wire = result.expect("URL image with detail=None should convert successfully");

        // @step And the resulting image_url.url equals the original URL
        // @step And the resulting image_url.detail equals "auto"
        match wire {
            UserContent::Image { image_url } => {
                assert_eq!(image_url.url, original_url);
                assert_eq!(image_url.detail, ImageDetail::Auto);
            }
            other => panic!("expected UserContent::Image, got {other:?}"),
        }
    }

    #[test]
    fn prov_083_base64_with_media_type_none_still_errors_on_mime() {
        // @step Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=None, and detail=None
        let rig_content = build_rig_image(
            DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg".into()),
            None,
            None,
        );

        // @step When the provider converts the message into an OpenAI chat completion Message
        let result = convert(rig_content);

        // @step Then the conversion returns a MessageError::ConversionError
        let err = result.expect_err("missing media_type must still error");
        match err {
            message::MessageError::ConversionError(msg) => {
                // @step And the error message references the missing MIME type (not the missing detail)
                let lower = msg.to_lowercase();
                assert!(
                    lower.contains("media type") || lower.contains("mime"),
                    "error should reference missing MIME/media type, got: {msg}"
                );
                assert!(
                    !lower.contains("image detail"),
                    "error should NOT be about missing image detail, got: {msg}"
                );
            }
        }
    }
}

// ================================================================
// PROV-084 TESTS — tool-returned images (Read/PDF/MCP)
//
// Feature: spec/features/openai-provider-tool-result-images.feature
//
// Covers PROV-084 — a rig `ToolResult` containing image parts must
// convert into an OpenAI `tool` message (carrying text or a
// placeholder) followed by a `user` message whose content array
// carries every `image_url` part. Text-only tool results must
// continue to convert into a single `tool` message.
// ================================================================
#[cfg(test)]
mod prov_084_tests {
    //! Feature: spec/features/openai-provider-tool-result-images.feature
    use super::*;
    use crate::OneOrMany;
    use crate::completion::message::{
        Message as RigMessage, ToolResult as RigToolResult,
        ToolResultContent as RigToolResultContent, UserContent as RigUserContent,
    };
    use crate::message::ImageMediaType;

    fn png_image_part() -> RigToolResultContent {
        RigToolResultContent::image_base64(
            "iVBORw0KGgoAAAANSUhEUg",
            Some(ImageMediaType::PNG),
            None,
        )
    }

    fn jpeg_image_part() -> RigToolResultContent {
        RigToolResultContent::image_base64(
            "/9j/4AAQSkZJRgABAQEAYABgAAD",
            Some(ImageMediaType::JPEG),
            None,
        )
    }

    fn text_part(text: &str) -> RigToolResultContent {
        RigToolResultContent::text(text)
    }

    /// Wrap a `ToolResult` into a rig `Message::User` and convert via the
    /// production `TryFrom<message::Message> for Vec<Message>` impl.
    fn convert_tool_result(tool_result: RigToolResult) -> Vec<Message> {
        let rig_msg = RigMessage::User {
            content: OneOrMany::one(RigUserContent::ToolResult(tool_result)),
        };
        <Vec<Message> as TryFrom<RigMessage>>::try_from(rig_msg)
            .expect("tool-result → openai Vec<Message> conversion should succeed")
    }

    #[test]
    fn prov_084_single_base64_image_emits_tool_then_user() {
        // @step Given a rig user message whose content is a `ToolResult` with id "call_abc", one `ToolResultContent::Image` (base64, media_type=image/png), and no text parts
        let tr = RigToolResult {
            id: "call_abc".into(),
            call_id: None,
            content: OneOrMany::one(png_image_part()),
        };

        // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
        let result = convert_tool_result(tr);

        // @step Then the resulting Vec<Message> has exactly 2 elements
        assert_eq!(result.len(), 2, "expected 2 messages, got {}", result.len());

        // @step And the first element is a `tool` role message with tool_call_id "call_abc"
        match &result[0] {
            Message::ToolResult {
                tool_call_id,
                content,
            } => {
                assert_eq!(tool_call_id, "call_abc");
                // @step And the first element's content is a non-empty placeholder string
                let text = content.as_text();
                assert!(!text.is_empty(), "tool content placeholder must be non-empty");
            }
            other => panic!("expected Message::ToolResult, got {other:?}"),
        }

        // @step And the second element is a `user` role message
        let user_json = match &result[1] {
            user @ Message::User { content, .. } => {
                assert!(
                    !content.is_empty(),
                    "user message must carry at least one content part"
                );
                serde_json::to_value(user).expect("serialize user message")
            }
            other => panic!("expected Message::User, got {other:?}"),
        };

        // @step And the second element contains exactly one `image_url` content part
        let content_array = user_json["content"]
            .as_array()
            .expect("user content should serialize as an array");
        assert_eq!(content_array.len(), 1, "expected exactly one content part");
        assert_eq!(content_array[0]["type"], "image_url");

        // @step And that image_url.url starts with "data:image/png;base64,"
        let url = content_array[0]["image_url"]["url"]
            .as_str()
            .expect("image_url.url must be a string");
        assert!(
            url.starts_with("data:image/png;base64,"),
            "unexpected url prefix: {url}"
        );

        // @step And that image_url.detail equals "auto"
        assert_eq!(content_array[0]["image_url"]["detail"], "auto");
    }

    #[test]
    fn prov_084_text_only_tool_result_still_single_message() {
        // @step Given a rig user message whose content is a `ToolResult` with id "call_text" and one `ToolResultContent::Text("hello")`
        let tr = RigToolResult {
            id: "call_text".into(),
            call_id: None,
            content: OneOrMany::one(text_part("hello")),
        };

        // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
        let result = convert_tool_result(tr);

        // @step Then the resulting Vec<Message> has exactly 1 element
        assert_eq!(result.len(), 1, "expected 1 message, got {}", result.len());

        // @step And the element is a `tool` role message with tool_call_id "call_text"
        // @step And the element's content equals "hello"
        match &result[0] {
            Message::ToolResult {
                tool_call_id,
                content,
            } => {
                assert_eq!(tool_call_id, "call_text");
                assert_eq!(content.as_text(), "hello");
            }
            other => panic!("expected Message::ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn prov_084_three_images_yield_tool_plus_user_with_three_parts() {
        // @step Given a rig user message whose content is a `ToolResult` with id "call_pdf" and three `ToolResultContent::Image` parts in page order (page-1, page-2, page-3)
        let page1 = RigToolResultContent::image_base64(
            "UEFHRS0xAAAA",
            Some(ImageMediaType::PNG),
            None,
        );
        let page2 = RigToolResultContent::image_base64(
            "UEFHRS0yAAAA",
            Some(ImageMediaType::PNG),
            None,
        );
        let page3 = RigToolResultContent::image_base64(
            "UEFHRS0zAAAA",
            Some(ImageMediaType::PNG),
            None,
        );
        let content = OneOrMany::many(vec![page1, page2, page3])
            .expect("three-part tool result content");
        let tr = RigToolResult {
            id: "call_pdf".into(),
            call_id: None,
            content,
        };

        // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
        let result = convert_tool_result(tr);

        // @step Then the resulting Vec<Message> has exactly 2 elements
        assert_eq!(result.len(), 2);

        // @step And the first element is a `tool` role message with tool_call_id "call_pdf"
        match &result[0] {
            Message::ToolResult { tool_call_id, .. } => {
                assert_eq!(tool_call_id, "call_pdf");
            }
            other => panic!("expected Message::ToolResult, got {other:?}"),
        }

        // @step And the second element is a `user` role message
        let user_json = match &result[1] {
            user @ Message::User { .. } => serde_json::to_value(user).expect("serialize user"),
            other => panic!("expected Message::User, got {other:?}"),
        };

        // @step And the second element contains exactly three `image_url` content parts in page order
        let parts = user_json["content"]
            .as_array()
            .expect("user content array");
        assert_eq!(parts.len(), 3, "expected three image parts");
        for part in parts {
            assert_eq!(part["type"], "image_url");
        }
        // Spot-check page ordering using the distinct base64 payloads.
        let url0 = parts[0]["image_url"]["url"].as_str().expect("url0");
        let url1 = parts[1]["image_url"]["url"].as_str().expect("url1");
        let url2 = parts[2]["image_url"]["url"].as_str().expect("url2");
        assert!(url0.contains("UEFHRS0xAAAA"), "page-1 expected first: {url0}");
        assert!(url1.contains("UEFHRS0yAAAA"), "page-2 expected second: {url1}");
        assert!(url2.contains("UEFHRS0zAAAA"), "page-3 expected third: {url2}");
    }

    #[test]
    fn prov_084_mixed_text_and_image_splits_correctly() {
        // @step Given a rig user message whose content is a `ToolResult` with id "call_mcp", one `ToolResultContent::Text("summary text")`, and one `ToolResultContent::Image` (base64, media_type=image/jpeg)
        let content = OneOrMany::many(vec![text_part("summary text"), jpeg_image_part()])
            .expect("mixed tool result content");
        let tr = RigToolResult {
            id: "call_mcp".into(),
            call_id: None,
            content,
        };

        // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
        let result = convert_tool_result(tr);

        // @step Then the resulting Vec<Message> has exactly 2 elements
        assert_eq!(result.len(), 2);

        // @step And the first element is a `tool` role message with tool_call_id "call_mcp"
        // @step And the first element's content contains the substring "summary text"
        match &result[0] {
            Message::ToolResult {
                tool_call_id,
                content,
            } => {
                assert_eq!(tool_call_id, "call_mcp");
                let text = content.as_text();
                assert!(
                    text.contains("summary text"),
                    "tool-message content must carry the text part, got: {text}"
                );
            }
            other => panic!("expected Message::ToolResult, got {other:?}"),
        }

        // @step And the second element is a `user` role message
        let user_json = match &result[1] {
            user @ Message::User { .. } => serde_json::to_value(user).expect("serialize user"),
            other => panic!("expected Message::User, got {other:?}"),
        };

        // @step And the second element contains exactly one `image_url` content part
        let parts = user_json["content"]
            .as_array()
            .expect("user content array");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["type"], "image_url");

        // @step And that image_url.url starts with "data:image/jpeg;base64,"
        let url = parts[0]["image_url"]["url"].as_str().expect("url string");
        assert!(
            url.starts_with("data:image/jpeg;base64,"),
            "unexpected url prefix: {url}"
        );
    }
}
