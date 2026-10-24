use base64::{engine::general_purpose::STANDARD, Engine};
use codelet_core::persistence::{DocumentSource, ImageSource, UserContent as StoredContent};
use codelet_rpc_types::{IncomingMessageImage, StreamChunk};
use rig::message::{Document, DocumentMediaType, DocumentSourceKind, ImageMediaType, UserContent};
use serde_json::{json, Value};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("invalid stored media metadata: {0}")]
    Metadata(String),
    #[error("unsupported stored media format: {0}")]
    Unsupported(String),
    #[error("stored media source is unavailable; resolve its blob before replay")]
    MissingSource,
}

fn base64_source(data: &str) -> Result<(), MediaError> {
    if data.starts_with(codelet_core::persistence::BLOB_REF_PREFIX) {
        return Err(MediaError::MissingSource);
    }
    if data.is_empty() || STANDARD.decode(data).is_err() {
        return Err(MediaError::Metadata("invalid or empty base64 data".into()));
    }
    Ok(())
}

fn remote_source(source: &str) -> Result<(), MediaError> {
    let url =
        url::Url::parse(source).map_err(|_| MediaError::Metadata("invalid external URL".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.has_host()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(MediaError::Metadata(
            "external media requires an HTTP or HTTPS URL without credentials".into(),
        ));
    }
    Ok(())
}

fn image_type(media_type: &str) -> Result<ImageMediaType, MediaError> {
    match media_type {
        "image/png" => Ok(ImageMediaType::PNG),
        "image/jpeg" | "image/jpg" => Ok(ImageMediaType::JPEG),
        "image/gif" => Ok(ImageMediaType::GIF),
        "image/webp" => Ok(ImageMediaType::WEBP),
        _ => Err(MediaError::Unsupported(media_type.into())),
    }
}

pub(crate) fn convert(block: &Value) -> Result<(UserContent, StreamChunk), MediaError> {
    let content: StoredContent = serde_json::from_value(block.clone())
        .map_err(|error| MediaError::Metadata(error.to_string()))?;
    match content {
        StoredContent::Image {
            source: ImageSource::Base64 { media_type, data },
        } => {
            base64_source(&data)?;
            let mime = image_type(&media_type)?;
            if codelet_tools::image_dimensions::extract_dimensions_from_base64(&data).is_some_and(
                |(width, height)| {
                    codelet_tools::image_dimensions::exceeds_pixel_limit(width, height)
                },
            ) {
                return Err(MediaError::Metadata("image exceeds pixel limit".into()));
            }
            let chunk = StreamChunk::incoming_message_with_images(
                format!("[Image: {media_type}]"),
                vec![IncomingMessageImage {
                    data: data.clone(),
                    media_type,
                }],
            );
            Ok((UserContent::image_base64(data, Some(mime), None), chunk))
        }
        StoredContent::Image {
            source: ImageSource::Url { url },
        } => {
            remote_source(&url)?;
            let chunk = StreamChunk::user_input(format!("[External image URL: {url}]"));
            Ok((UserContent::image_url(url, None, None), chunk))
        }
        StoredContent::Document {
            source,
            title,
            context,
            cache_control,
        } => {
            let data = match source {
                DocumentSource::Base64 { media_type, data } => {
                    if media_type != "application/pdf" {
                        return Err(MediaError::Unsupported(media_type));
                    }
                    base64_source(&data)?;
                    DocumentSourceKind::Base64(data)
                }
                DocumentSource::Url { url } => {
                    remote_source(&url)?;
                    DocumentSourceKind::Url(url)
                }
            };
            let label = title.as_deref().unwrap_or("PDF document");
            let description = match &data {
                DocumentSourceKind::Url(url) => format!("[External document URL: {url}; {label}]"),
                _ => format!("[Document: application/pdf; {label}]"),
            };
            Ok((
                UserContent::Document(Document {
                    data,
                    media_type: Some(DocumentMediaType::PDF),
                    additional_params: Some(json!({
                        "title":title,"context":context,"cache_control":cache_control,
                    })),
                }),
                StreamChunk::user_input(description),
            ))
        }
        _ => Err(MediaError::Unsupported("not an image or document".into())),
    }
}
