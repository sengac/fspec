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
