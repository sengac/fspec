//! Markdown rendering, viewer HTML template, and HTML escaping.

mod autolink;
mod escape;
mod render;
mod slug;
mod template;

pub use escape::html_escape;
pub use render::render_markdown;
pub use template::viewer_template;
