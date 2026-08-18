//! TUI-106 — shared animated loading dialog base.
//!
//! Feature: spec/features/shared-animated-loadingdialog-base-reusing-the-canonical-dialog-theme-with-lifted-braille-spinner-redraw-clock-gate.feature
//!
//! Pattern B (view-owned modal) built on the shared `dialog_theme`
//! base: the lazy mode-views (Checkpoints — TUI-107, Changed Files —
//! TUI-108) own a [`LoadingDialog`] + a [`super::load_state::LoadTracker`]
//! while their cascade RPCs are in flight; this module supplies the
//! pixel contract layered over `dialog_theme::render_dialog` — the
//! SAME single implementation the rest of the dialog stack uses
//! (identical to how `StatusDialog` delegates). The animated spinner
//! line reuses the lifted braille spinner from
//! [`super::spinner::current_frame_glyph`].
//!
//! Contract:
//! - rounded border in `Accent::Cyan` (the shared-base look);
//! - one body row `"{glyph} {label}"` (glyph advances with elapsed ms);
//! - a second `(idx/total)` counter row ONLY once a caller feeds
//!   progress (the TUI-109 wire hook — `set_progress`);
//! - an empty footer;
//! - `dismissable()` is false: ESC is ignored while loading
//!   (StatusDialog rule [7] surfaced through the view key routing).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::spinner::current_frame_glyph;

/// Stateless modal value owned by one lazy mode-view while a cascade
/// load is in flight. Present ⇒ painting over the panes via
/// [`render_loading_dialog`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadingDialog {
    /// Bold title row, e.g. "Loading checkpoints".
    pub title: String,
    /// Stable per-stage spinner-line label, e.g.
    /// "Loading checkpoint list…", "Loading files for {name}…".
    pub label: String,
    /// Optional `(idx, total)` counter fed by the TUI-109 wire hook.
    /// Absent (`None`) ⇒ the `(idx/total)` row is not painted.
    pub progress: Option<(usize, usize)>,
}

impl LoadingDialog {
    /// Construct a dialog in the list/scan stage.
    pub fn new(title: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            label: label.into(),
            progress: None,
        }
    }

    /// TUI-109 hook: feed a per-item `(idx, total)` counter. No-op for
    /// the render while `None`; once set the dialog gains the
    /// `"(idx/total)"` row.
    pub fn set_progress(&mut self, idx: usize, total: usize) {
        self.progress = Some((idx, total));
    }

    /// Loading dialogs are NEVER dismissible while the load is in
    /// flight (StatusDialog rule [7]: ESC ignored in `Restoring`).
    /// View key routing consults this and returns Ignored.
    pub fn dismissable(&self) -> bool {
        false
    }

    /// The animated spinner line: `"{glyph} {label}"` where the glyph
    /// comes from the lifted braille spinner (80 ms cadence, frame =
    /// `elapsed / 80 % 10`).
    pub fn spinner_line(&self, elapsed_ms: u64) -> String {
        format!("{} {}", current_frame_glyph(elapsed_ms), self.label)
    }
}

/// Paint `dialog` over `area`/`buf` (the pattern-B paint-over-the-panes
/// helper): builds an `FspecDialog { accent: Cyan, title,
/// rows: [spinner line, optional "(idx/total)"], footer: "",
/// min_width: 40 }` and delegates the pixel paint to the single shared
/// [`render_dialog`] implementation.
pub fn render_loading_dialog(area: Rect, buf: &mut Buffer, dialog: &LoadingDialog, elapsed_ms: u64) {
    let mut rows = vec![DialogRow {
        spans: vec![Span::raw(dialog.spinner_line(elapsed_ms))],
        selectable: false,
        selected: false,
    }];
    if let Some((idx, total)) = dialog.progress {
        // TUI-109: the total is only known after enumeration completes,
        // so a pending total (0) renders as "(idx/…)".
        let counter = if total == 0 {
            format!("({idx}/…)")
        } else {
            format!("({idx}/{total})")
        };
        rows.push(DialogRow {
            spans: vec![Span::raw(counter)],
            selectable: false,
            selected: false,
        });
    }
    let spec = FspecDialog {
        accent: Accent::Cyan,
        title: &dialog.title,
        rows,
        footer: "",
        min_width: 40,
    };
    render_dialog(area, buf, &spec);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::style::Color;
    use super::*;

    fn paint(dialog: &LoadingDialog, elapsed: u64) -> (Buffer, Rect) {
        let area = Rect::new(0, 0, 60, 14);
        let mut buf = Buffer::empty(area);
        render_loading_dialog(area, &mut buf, dialog, elapsed);
        (buf, area)
    }

    fn text(buf: &Buffer) -> String {
        let out: String = buf.content.iter().map(|c| c.symbol().to_string()).collect();
        out
    }

    #[test]
    fn fresh_dialog_shows_title_glyph_and_label() {
        let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
        let (buf, _) = paint(&dialog, 0);
        let out = text(&buf);
        assert!(out.contains("Loading checkpoints"), "title");
        assert!(out.contains("⠋"), "first glyph at t=0");
        assert!(out.contains("Loading checkpoint list…"), "stage label");
    }

    #[test]
    fn counter_row_appears_only_with_progress() {
        let mut dialog =
            LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
        let out = text(&paint(&dialog, 0).0);
        assert!(!out.contains("(/"), "no counter row without progress");
        dialog.set_progress(3, 10);
        let out = text(&paint(&dialog, 0).0);
        assert!(out.contains("(3/10)"), "counter row once progress is fed");
    }

    #[test]
    fn border_is_rounded_accent_cyan() {
        let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
        let (buf, _) = paint(&dialog, 0);
        let corner: Vec<_> = buf
            .content
            .iter()
            .filter(|c| matches!(c.symbol(), "╭" | "╮" | "╰" | "╯"))
            .collect();
        assert!(!corner.is_empty(), "corner glyphs present");
        let style = corner[0].style();
        assert_eq!(style.fg, Some(Accent::Cyan.color()), "border fg cyan");
        assert_eq!(style.bg, Some(Color::Black), "border bg black");
    }

    #[test]
    fn spinning_glyph_advances_between_0_and_80_ms() {
        let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
        let out0 = text(&paint(&dialog, 0).0);
        let out80 = text(&paint(&dialog, 80).0);
        assert!(out0.contains("⠋") && !out0.contains("⠙"), "t=0 → first glyph only");
        assert!(out80.contains("⠙"), "t=80 → second glyph");
    }

    #[test]
    fn loading_dialog_is_never_dismissable() {
        let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
        assert!(!dialog.dismissable(), "never dismissible while loading");
    }
}
