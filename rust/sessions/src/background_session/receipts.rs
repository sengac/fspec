//! BUG-284: retain each submission as a durable Pending receipt before its
//! queue acknowledgment. Queue refusal settles a truthful non-admitted
//! receipt; storage failure is returned to the caller as a refusal.

use super::{BackgroundSession, BridgeImageData};
use codelet_core::persistence::input_receipts::{
    self, InputDisposition, InputOrigin, InputReceipt,
};
use codelet_core::persistence::{ImageSource, UserContent};
use uuid::Uuid;

/// The canonical original content shared by receipts and admitted messages.
pub fn input_content(text: &str, images: &[BridgeImageData]) -> Vec<UserContent> {
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
    content
}

impl BackgroundSession {
    /// Durably retain a Pending submission and return its immutable identity.
    pub fn retain_submission(
        &self,
        origin: InputOrigin,
        epoch: Option<u64>,
        text: &str,
        images: &[BridgeImageData],
    ) -> Result<Uuid, String> {
        let receipt = InputReceipt::pending(self.id, origin, epoch, input_content(text, images))
            .map_err(|error| format!("Failed to retain input: {error}"))?;
        input_receipts::retain(&receipt)
            .map_err(|error| format!("Failed to retain input: {error}"))?;
        Ok(receipt.id)
    }

    /// Record that a retained submission was refused before admission.
    pub fn refuse_submission(&self, id: Uuid, reason: &str) {
        let refused = InputDisposition::Refused {
            reason: reason.into(),
        };
        if let Err(error) = input_receipts::settle(self.id, id, refused) {
            tracing::error!(session_id = %self.id, submission = %id, %error, "refused submission receipt remains unresolved");
        }
    }
}
