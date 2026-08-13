//! Image dimension extraction from raw headers
//!
//! Re-exports from codelet-common::image_dimensions — the single source of truth.
//! This module exists for backwards compatibility so codelet-tools consumers
//! can continue to use `codelet_tools::image_dimensions::*`.
//!
//! Feature: spec/features/image-dimension-validation.feature

pub use codelet_common::image_dimensions::{
    check_image_dimensions, exceeds_pixel_limit, extract_dimensions_from_base64,
    extract_jpeg_dimensions, extract_png_dimensions, format_dimension_error,
    MAX_IMAGE_PIXEL_DIMENSION,
};
