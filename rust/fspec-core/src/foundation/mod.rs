//! Foundation discovery shared guidance (DISC-003).
//!
//! Single source of truth for the 8-field draft table, the full-field scan,
//! progress/trailer rendering, and the unified field reminder. Moves the
//! logic previously duplicated in `commands/update_foundation.rs` and
//! `commands/discover_foundation.rs` (rule 7/8 of the work unit).

pub mod guidance;
