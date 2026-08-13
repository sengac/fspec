//! HTTP request handlers + the path-validation guard.

mod health;
mod path;
mod view;

pub use health::health;
pub use path::validate_path;
pub use view::view;
