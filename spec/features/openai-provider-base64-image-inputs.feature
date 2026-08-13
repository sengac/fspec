@done
@PROV-083
@providers
@rust
@rig
@bug-fix
@multimodal
@image
Feature: OpenAI Chat Completions rejects base64 user images with detail=None
  """
  Fix location: rust/patches/rig-core/src/providers/openai/completion/mod.rs line 445-447. Current code returns ConversionError if detail is None. Replace `let detail = detail.ok_or(...)?` with `let detail = detail.unwrap_or_default();` to match the URL branch at line 431. `ImageDetail::default()` = `Auto` (verified at completion/mod.rs:218-228 where the enum is defined with #[serde(rename_all="lowercase")]). No other sites changed.
  Tests live alongside existing PROV-081 tests in rust/patches/rig-core/src/providers/openai/completion/mod.rs (`#[cfg(test)] mod prov_083_tests`). Each test deserializes or constructs a rig message, runs it through `<Vec<openai::Message> as TryFrom<rig::Message>>::try_from`, and asserts either (a) success with the expected serialized `image_url.url` / `image_url.detail`, or (b) the specific pre-existing error for negative cases (media_type=None path).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Base64 UserContent::Image with detail=None must convert successfully into an OpenAI `image_url` content part whose `detail` defaults to Auto (mirrors the URL path behaviour at completion/mod.rs:431 and OpenAI's own default).
  #   2. Base64 UserContent::Image with detail=Some(Low|High|Auto) must still honour the explicit value (backward-compat — existing callers that set detail must not regress).
  #   3. The resulting OpenAI content part must use `data:<mime>;base64,<payload>` as the `image_url.url` with the supplied `media_type` — no truncation, no re-encoding, no extra prefix.
  #   4. If `media_type` is None, the existing ConversionError behaviour stays — we are only fixing the missing-detail case, not guessing MIME types.
  #
  # EXAMPLES:
  #   1. A user pastes an inline PNG into the CLI bridge. codelet builds a UserContent::Image with base64 data, media_type=image/png, detail=None. The Chat Completion request is POSTed successfully to the OpenAI-compatible server with `image_url.detail="auto"`.
  #   2. A user pastes an inline JPEG with media_type=image/jpeg and detail=None — the server describes the image; no conversion error.
  #   3. A user attaches a base64 image and explicitly requests high-detail processing (detail=High). The request is POSTed with `image_url.detail="high"` — explicit values are preserved.
  #   4. A user attaches a URL image (not base64). Behaviour is unchanged — URL path already defaulted detail correctly, no regression from this fix.
  #   5. A user attaches a base64 image but codelet failed to detect the media type (media_type=None, detail=None). Today this errors with "image MIME type required"; that behaviour is PRESERVED — we only fix the detail case, not the media-type case.
  #
  # ========================================
  Background: User Story
    As a codelet user attaching an inline image (paste, bridge/Telegram, or future TUI multimodal) to a chat with a vision-capable OpenAI-compatible model
    I want to have the image reach the server and be described by the model
    So that the image-attach UX in codelet stops silently no-op'ing against vLLM / Qwen / any OpenAI-compatible vision server

  @unit
  @rust
  Scenario: Base64 user image with detail=None defaults to image_url.detail="auto"
    Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=image/png, and detail=None
    When the provider converts the message into an OpenAI chat completion Message
    Then the conversion succeeds
    And the resulting user content part has type "image_url"
    And the resulting image_url.url starts with "data:image/png;base64,"
    And the resulting image_url.detail equals "auto"

  @unit
  @rust
  Scenario: Base64 user image with detail=None and image/jpeg also converts successfully
    Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=image/jpeg, and detail=None
    When the provider converts the message into an OpenAI chat completion Message
    Then the conversion succeeds
    And the resulting image_url.url starts with "data:image/jpeg;base64,"
    And the resulting image_url.detail equals "auto"

  @unit
  @rust
  @backward-compat
  Scenario: Base64 user image with explicit detail=High preserves the explicit value
    Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=image/png, and detail=High
    When the provider converts the message into an OpenAI chat completion Message
    Then the conversion succeeds
    And the resulting image_url.detail equals "high"

  @unit
  @rust
  @regression
  Scenario: URL image path behaviour is unchanged
    Given a caller builds a rig `message::UserContent::Image` with a https URL data source, media_type=image/png, and detail=None
    When the provider converts the message into an OpenAI chat completion Message
    Then the conversion succeeds
    And the resulting image_url.url equals the original URL
    And the resulting image_url.detail equals "auto"

  @unit
  @rust
  @error
  Scenario: Base64 user image with media_type=None still errors (out of scope for this fix)
    Given a caller builds a rig `message::UserContent::Image` with base64 data, media_type=None, and detail=None
    When the provider converts the message into an OpenAI chat completion Message
    Then the conversion returns a MessageError::ConversionError
    And the error message references the missing MIME type (not the missing detail)
