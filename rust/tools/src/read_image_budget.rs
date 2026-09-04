//! PROV-144: per-session Read-tool image-budget enforcement.
//!
//! The Read tool may return images in three shapes: a single image file, a
//! PDF read in `visual` mode (one image per page), and a PDF read in `images`
//! mode (one image per embedded image). This module is the single source of
//! truth for how many of those a single tool result may return, resolved from
//! the per-session registry populated by the sessions layer
//! ([`crate::model_capabilities::session_model_max_images`]).
//!
//! Budget semantics:
//! * entry **absent** (non-profile / unregistered session) → default of 4
//! * **`0`** (no-vision profile) → any read that WOULD return an image fails
//!   with a no-vision message pointing at text alternatives
//! * **`n >= 1`** → a read returning more than `n` images fails with a
//!   message naming the limit, the requested count, and how to read fewer
//!   (offset/limit) or raise the profile's Max Images setting
//!
//! Lives in its own module (not `read.rs`, already over the 300-LoC ceiling)
//! so both front doors consult the identical gate: the rig-native
//! `ReadTool::call` and the `FileToolFacadeWrapper` path (which delegates to
//! `ReadTool::call`).

use uuid::Uuid;

use crate::error::ToolError;
use crate::model_capabilities;

/// Default image budget applied when a session has no registered entry
/// (non-profile or unregistered sessions).
pub const DEFAULT_MAX_IMAGES: u32 = 4;

/// Resolve the session's effective image budget: the registered value, or
/// [`DEFAULT_MAX_IMAGES`] when no entry is registered.
pub fn effective_budget(session_id: Uuid) -> u32 {
    model_capabilities::session_model_max_images(session_id).unwrap_or(DEFAULT_MAX_IMAGES)
}

/// The no-vision (budget `0`) failure message.
///
/// Names the `0` budget, states image reading is disabled, and points at
/// text-based alternatives (`pdf_mode='text'`, Grep, Read on text files).
pub fn no_vision_error() -> ToolError {
    ToolError::Validation {
        tool: "read",
        message: "Image reading is disabled: this session's profile is configured with \
                  Max Images = 0 (no vision model). Use text-based tools instead \
                  (read PDFs with pdf_mode='text', or use Grep/Read on text files)."
            .to_string(),
    }
}

/// The over-budget failure message.
///
/// Names the limit, the requested count, and how to read fewer (offset/limit)
/// or raise the profile's Max Images setting.
pub fn over_budget_error(budget: u32, requested: usize, unit: &str) -> ToolError {
    ToolError::Validation {
        tool: "read",
        message: format!(
            "This session allows at most {budget} image(s) per Read result (profile Max \
             Images limit). This read would return {requested} {unit}, exceeding the limit. \
             Use offset/limit to read at most {budget} {unit} at a time, or raise the \
             profile's Max Images setting."
        ),
    }
}

/// Enforce the budget for a read that would return `requested` images.
///
/// * budget `0` → the no-vision error (any image read fails).
/// * `requested > budget` → the over-budget error.
/// * otherwise → `Ok(())`.
///
/// A single image-file read calls this with `requested = 1`, so it always
/// passes for any budget `>= 1` and always fails for budget `0`.
pub fn check_image_count(budget: u32, requested: usize, unit: &str) -> Result<(), ToolError> {
    if budget == 0 {
        return Err(no_vision_error());
    }
    if (requested as u32) > budget {
        return Err(over_budget_error(budget, requested, unit));
    }
    Ok(())
}

/// Gate a PDF read's image-returning mode against the session's image budget.
///
/// `mode` is the EFFECTIVE mode after the vision/no-vision fallback:
/// * `"text"` → no gate (text never returns images).
/// * `"visual"` / `"images"` on budget `0` → the no-vision error (the caller
///   explicitly asked for images; a budget-0 session cannot see them).
/// * otherwise → the over-budget check against the number of images the read
///   would return (rendered pages / embedded images). Returns `Ok(())`
///   when the PDF cannot be loaded or is encrypted — the normal read path
///   surfaces the load error itself.
pub fn gate_pdf_image_mode(
    budget: u32,
    bytes: &[u8],
    offset: usize,
    limit: usize,
    mode: &str,
) -> Result<(), ToolError> {
    if budget == 0 {
        return if matches!(mode, "visual" | "images") {
            Err(no_vision_error())
        } else {
            Ok(())
        };
    }
    match mode {
        "visual" => {
            if let Some(requested) = pdf_pages_to_render(bytes, offset, limit) {
                check_image_count(budget, requested, "pages")?;
            }
        }
        "images" => {
            if let Some(requested) = pdf_images_to_extract(bytes, offset, limit) {
                check_image_count(budget, requested, "images")?;
            }
        }
        _ => {} // "text" never returns images
    }
    Ok(())
}

/// Number of pages a `visual`-mode read with the given 1-based `offset` and
/// `limit` would render (the clamped range length, mirroring
/// [`crate::pdf::render_pdf_pages`]). `None` when the PDF cannot be loaded or
/// is encrypted — the normal read path then surfaces the load error itself.
pub fn pdf_pages_to_render(bytes: &[u8], offset: usize, limit: usize) -> Option<usize> {
    use lopdf::Document;
    let doc = Document::load_mem(bytes).ok()?;
    if doc.is_encrypted() {
        return None;
    }
    let total = doc.get_pages().len();
    let start = offset.max(1);
    if start > total {
        return Some(0);
    }
    let end = start.saturating_add(limit.saturating_sub(1)).min(total);
    Some(end - start + 1)
}

/// Number of embedded images an `images`-mode read with the given 1-based
/// `offset` and `limit` would extract (the count of image XObjects whose
/// 1-based document-wide index falls in the range, mirroring
/// [`crate::pdf::extract_pdf_images`]). `None` when the PDF cannot be loaded
/// or is encrypted.
pub fn pdf_images_to_extract(bytes: &[u8], offset: usize, limit: usize) -> Option<usize> {
    use lopdf::Document;
    let doc = Document::load_mem(bytes).ok()?;
    if doc.is_encrypted() {
        return None;
    }
    let start = offset.max(1) as u32;
    let end = start.saturating_add(limit.saturating_sub(1) as u32);
    let mut image_index = 0u32;
    let mut count = 0usize;
    for (_object_id, object) in doc.objects.iter() {
        if let Ok(stream) = object.as_stream() {
            let is_image = stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|s| s.as_name().ok())
                .map(|n| n == b"Image")
                .unwrap_or(false);
            if !is_image {
                continue;
            }
            image_index += 1;
            if image_index >= start && image_index <= end {
                count += 1;
            }
        }
    }
    Some(count)
}
