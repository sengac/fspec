//! Streaming Token Display Utilities
//!
//! This module provides composable utilities for tracking and displaying token
//! information during streaming responses from LLM providers.
//!
//! ## Architecture
//!
//! The streaming display system handles the complexity of showing token counts
//! during streaming, where:
//!
//! - **Anthropic/Gemini**: Send `Usage` events during streaming (authoritative values mid-stream)
//! - **OpenAI/Z.AI**: Only send usage in `FinalResponse` (need estimates during stream)
//!
//! ## Components
//!
//! - [`OutputTokenTracker`]: Tracks output tokens with explicit estimated vs authoritative states
//! - [`TokPerSecCalculator`]: Calculates tokens-per-second rate with EMA smoothing
//! - [`DisplayThrottle`]: Throttles UI updates to prevent flicker
//! - [`StreamingTokenDisplay`]: Composes all components for easy use in stream loops
//!
//! ## Usage
//!
//! ```ignore
//! use codelet_core::streaming_display::StreamingTokenDisplay;
//!
//! let mut display = StreamingTokenDisplay::new(prev_input, prev_output, cache_read, cache_creation);
//!
//! // During streaming - record text chunks
//! if let Some(update) = display.record_chunk(&text) {
//!     output.emit_tokens(&update.into());
//! }
//!
//! // When Usage event arrives (output > 0)
//! if let Some(update) = display.update_from_usage(&usage) {
//!     output.emit_tokens(&update.into());
//! }
//!
//! // When new API segment starts (MessageStart with output == 0)
//! display.start_new_segment(&usage);
//! ```

mod display_throttle;
mod output_token_tracker;
mod streaming_token_display;
mod tok_per_sec_calculator;

pub use display_throttle::DisplayThrottle;
pub use output_token_tracker::OutputTokenTracker;
pub use streaming_token_display::{StreamingTokenDisplay, TokenDisplayUpdate};
pub use tok_per_sec_calculator::TokPerSecCalculator;
