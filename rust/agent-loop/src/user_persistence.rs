use codelet_core::persistence::{ImageSource, UserContent};
use codelet_sessions::background_session::BridgeImageData;

#[derive(Debug, thiserror::Error)]
pub enum UserPersistenceError {
    #[error("user content persistence failed: {0}")]
    Store(String),
}

pub fn persist_user_input(
    session_id: &uuid::Uuid,
    text: &str,
    images: &[BridgeImageData],
) -> Result<(), UserPersistenceError> {
    let mut content = Vec::with_capacity(images.len().saturating_add(1));
    if !text.is_empty() || images.is_empty() {
        content.push(UserContent::Text { text: text.into() });
    }
    content.extend(images.iter().map(|image| UserContent::Image {
        source: ImageSource::Base64 {
            media_type: image.media_type.clone(),
            data: image.data.clone(),
        },
    }));
    crate::persist::persist_user_content(session_id, content).map_err(UserPersistenceError::Store)
}
