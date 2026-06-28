//! Viewer HTML template — port of `src/server/templates/viewer-template.ts`.
//!
//! Assembles a full HTML document: escaped basename in `<title>`, the
//! `prism-vsc-dark-plus` theme stylesheet, the mermaid module loader + Prism
//! core/autoloader scripts ([`scripts::HEAD_SCRIPTS`]), the themed stylesheet
//! ([`styles::STYLES`]), and a body with the theme-toggle control, font-size
//! controls, the `.markdown-content` wrapper, and the interaction script
//! ([`scripts::INTERACTION_SCRIPT`]).

mod mermaid_modal;
mod mermaid_wheel;
mod modal_styles;
mod scripts;
mod styles;

use super::escape::html_escape;

const PRISM_THEME_CSS: &str =
    "https://cdnjs.cloudflare.com/ajax/libs/prism-themes/1.9.0/prism-vsc-dark-plus.min.css";

/// Build the full HTML viewer document for a rendered markdown fragment.
pub fn viewer_template(title: &str, content_html: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"UTF-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
  <title>{title}</title>\n\
  <link rel=\"stylesheet\" href=\"{prism_css}\" />\n\
{head_scripts}\n\
  <style>{styles}{modal_styles}</style>\n\
</head>\n\
<body>\n\
  <button id=\"theme-toggle\" class=\"theme-toggle\"><span id=\"theme-icon\">\u{1F319}</span></button>\n\
  <div id=\"font-size-controls\" class=\"font-size-controls\">\n\
    <button id=\"font-size-decrease\" class=\"font-size-button\">\u{2212}</button>\n\
    <span id=\"font-size-display\" class=\"font-size-display\">16px</span>\n\
    <button id=\"font-size-increase\" class=\"font-size-button\">+</button>\n\
  </div>\n\
  <div class=\"markdown-content\">\n{content}\n  </div>\n\
{modal_markup}\n\
{panzoom_cdn}\n\
{interaction}\n\
{modal_script}\n\
</body>\n\
</html>\n",
        title = html_escape(title),
        prism_css = PRISM_THEME_CSS,
        head_scripts = scripts::HEAD_SCRIPTS,
        styles = styles::STYLES,
        modal_styles = modal_styles::MODAL_STYLES,
        content = content_html,
        modal_markup = mermaid_modal::MODAL_MARKUP,
        panzoom_cdn = mermaid_modal::PANZOOM_CDN,
        interaction = scripts::INTERACTION_SCRIPT,
        modal_script = mermaid_modal::modal_script(),
    )
}
