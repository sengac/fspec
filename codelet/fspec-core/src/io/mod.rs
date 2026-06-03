//! Shared I/O modules used by every ported fspec command.
//!
//! - [`project_root`] — find or create the project's `spec/` directory.
//! - [`locked_file`] — atomic JSON read/write with file locking.
//! - [`ensure`] — load-or-init helpers for canonical fspec state files
//!   (`work-units.json`, `prefixes.json`, …).
//!
//! All helpers are deliberately small and dependency-light so they can be
//! freely shared across the 162 child cards under RPC-003 without coupling
//! command logic to a single I/O backend.

pub mod ensure;
pub mod locked_file;
pub mod project_root;
