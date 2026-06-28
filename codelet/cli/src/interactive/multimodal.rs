//! BRIDGE-007 / EXT-016: Multimodal content building.
//!
//! BridgeImage struct for bridge image data and user content building
//! with pixel dimension validation.

use rig::message::{ImageMediaType, UserContent};
use rig::one_or_many::OneOrMany;
use tracing::warn;

/// BRIDGE-007: Image data from bridge for multimodal support
#[derive(Clone)]
pub struct BridgeImage {
    /// Base64-encoded image data
    pub data: String,
    /// Media type (e.g., "image/jpeg", "image/png")
    pub media_type: String,
}

/// EXT-016: Build user message content from prompt text and optional bridge images.
///
/// Validates pixel dimensions on each image before including it. Oversized images
/// are replaced with a text error message (Layer 3 defense-in-depth).
///
/// This function is public for testing.
pub fn build_user_content_with_images(
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
) -> OneOrMany<UserContent> {
    match images {
        Some(bridge_images) => {
            let mut content_parts: Vec<UserContent> = Vec::new();
            if !prompt.is_empty() {
                content_parts.push(UserContent::text(prompt));
            }
            for img in bridge_images {
                let media_type = match img.media_type.as_str() {
                    "image/jpeg" | "image/jpg" => Some(ImageMediaType::JPEG),
                    "image/png" => Some(ImageMediaType::PNG),
                    "image/gif" => Some(ImageMediaType::GIF),
                    "image/webp" => Some(ImageMediaType::WEBP),
                    _ => Some(ImageMediaType::JPEG),
                };

                // EXT-016: Validate pixel dimensions before adding user-pasted images
                // This is Layer 3 defense-in-depth — prevents oversized bridge images
                // from entering conversation history
                if let Some((width, height)) =
                    codelet_tools::image_dimensions::extract_dimensions_from_base64(&img.data)
                {
                    if codelet_tools::image_dimensions::exceeds_pixel_limit(width, height) {
                        let error_msg = codelet_tools::image_dimensions::format_dimension_error(
                            None, width, height,
                        );
                        warn!(
                            "Rejecting user-pasted image: {}x{} exceeds limit",
                            width, height
                        );
                        // Add error as text instead of the image
                        content_parts.push(UserContent::text(error_msg));
                        continue;
                    }
                }

                content_parts.push(UserContent::image_base64(img.data, media_type, None));
            }
            OneOrMany::many(content_parts)
                .unwrap_or_else(|_| OneOrMany::one(UserContent::text(prompt)))
        }
        None => OneOrMany::one(UserContent::text(prompt)),
    }
}
