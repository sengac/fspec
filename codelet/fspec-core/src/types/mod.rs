//! Shared type definitions for ported fspec commands.
//!
//! Every type here is intended to be reused by multiple command ports —
//! command-specific data shapes belong inside the individual command module.

pub mod coverage;
pub mod epic;
pub mod prefix;
pub mod tags;
pub mod work_unit;
