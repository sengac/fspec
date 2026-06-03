//! Feature: spec/features/provider-settings-list-inline-testresult-rendering-on-focused-row.feature
//!
//! Source-shape regression tests for RPC-150. Pins the canonical
//! paint site of the inline test_result decoration in
//! `codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs`
//! so the RPC-072 stub state (test result hidden inside the now-
//! removed `Detail::Summary` view) cannot silently re-emerge in
//! list-mode rendering.
//!
//!   * `render_nav_items` reads `view.test_result.as_ref()` inside
//!     its per-row paint loop, gated by
//!     `matches!(kind, RowKind::Provider { .. })` AND
//!     `test_result.provider_id == item.provider_id` — only the
//!     matching Provider row carries the ladder.
//!   * `paint_test_result_decoration` exists exactly once as a
//!     definition AND exactly once as a (multi-line) call site.
//!   * Its canonical signature has six parameters (`kind`, `selected`,
//!     `status`, `row_area`, `end_x`, `buf`).
//!   * The helper is exclusively owned by `list_nav_render.rs` —
//!     `detail.rs` and `row_render.rs` must contain zero references.
//!   * Body computes the right boundary via
//!     `row_area.x.saturating_add(row_area.width)`, reserves one
//!     separator cell (`separator_x = end_x`), advances by one
//!     (`decoration_x = end_x.saturating_add(1)`), and early-returns
//!     on both `end_x >= right_bound` and `decoration_x >= right_bound`.
//!   * Foreground from `status.decoration()`, background from
//!     `row_band_bg(kind, selected)`, composed via
//!     `Style::default().fg(fg).bg(bg)`.
//!
//! These tests run in sub-milliseconds — they only read source
//! strings; no ratatui `Buffer` / `Frame` is constructed. The
//! behavioural state contract (set_test_result / clear_test_result
//! / field default) is covered by `rpc158` tests; the on-arrow-nav
//! clear behaviour by `rpc159`. RPC-150 complements those with
//! render-only structural pinning so a regression breaks the test
//! before reaching CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn read_view(rel: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, "src", "views", "provider_settings", rel]
        .iter()
        .collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn list_nav_render_rs() -> String {
    read_view("list_nav_render.rs")
}

fn detail_rs() -> String {
    read_view("detail.rs")
}

fn row_render_rs() -> String {
    read_view("row_render.rs")
}

/// Walk braces forward from `abs_open` (which must be the index of an
/// opening `{`) until the matching close, returning the inclusive
/// substring.
fn brace_balanced(src: &str, abs_open: usize) -> &str {
    let bytes = src.as_bytes();
    assert!(bytes[abs_open] == b'{', "abs_open must point at `{{`");
    let mut depth: i32 = 0;
    let mut i = abs_open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[abs_open..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("matching closing brace for opener at {abs_open} not found");
}

/// Extract the body of the named function. Matches on the signature
/// substring `fn <name>(` so it works for both single-line and
/// multi-line signatures.
fn fn_body<'a>(src: &'a str, fn_name: &str) -> &'a str {
    let needle = format!("fn {fn_name}(");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`fn {fn_name}(` not found"));
    let brace_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for `fn {fn_name}` not found"));
    brace_balanced(src, start + brace_rel)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: render_nav_items reads view.test_result inside the per-row
//           paint loop
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_render_nav_items_reads_view_test_result_inside_the_per_row_paint_loop() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    let src = list_nav_render_rs();

    // @step When I extract the brace-balanced body of the "fn render_nav_items" function
    let body = fn_body(&src, "render_nav_items");

    // @step Then the body must contain "view.test_result.as_ref()"
    assert!(
        body.contains("view.test_result.as_ref()"),
        "render_nav_items body must read `view.test_result.as_ref()` so the inline ladder is driven by the latest dispatch result — without this, the decoration would never appear (RPC-150)"
    );

    // @step And the body must contain "for (row_idx, item) in nav_items"
    assert!(
        body.contains("for (row_idx, item) in nav_items"),
        "render_nav_items body must iterate `for (row_idx, item) in nav_items[...]` — the per-row loop is where the decoration is composited; moving the check outside the loop would either always or never paint the ladder (RPC-150)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Provider row gate guards the test_result decoration paint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_provider_row_gate_guards_the_test_result_decoration_paint() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    let src = list_nav_render_rs();

    // @step When I extract the brace-balanced body of the "fn render_nav_items" function
    let body = fn_body(&src, "render_nav_items");

    // @step Then the body must contain "matches!(kind, RowKind::Provider"
    let row_gate = body.find("matches!(kind, RowKind::Provider").expect(
        "render_nav_items body must guard the decoration paint with `matches!(kind, RowKind::Provider { .. })` so the ladder only ever shows on Provider rows — child rows (api-key, oauth-login, etc.) must NEVER carry the ladder (RPC-150)",
    );

    // @step And the body must contain "test_result.provider_id == item.provider_id"
    let id_gate = body.find("test_result.provider_id == item.provider_id").expect(
        "render_nav_items body must guard the decoration paint with `test_result.provider_id == item.provider_id` — without this every Provider row would paint the same status, smearing the last test across the list (RPC-150)",
    );

    // @step And the offset of "matches!(kind, RowKind::Provider" must be less than the offset of "test_result.provider_id == item.provider_id"
    assert!(
        row_gate < id_gate,
        "the RowKind::Provider gate MUST appear BEFORE the provider_id equality gate — the outer gate cheaply rejects non-Provider rows so the inner equality check is only done on Provider rows (RPC-150). row_gate={row_gate}, id_gate={id_gate}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: paint_test_result_decoration has exactly one call site and
//           exactly one fn definition
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_paint_test_result_decoration_has_exactly_one_call_and_one_definition() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    let src = list_nav_render_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the file must contain exactly one occurrence of "fn paint_test_result_decoration("
    let def_count = src.matches("fn paint_test_result_decoration(").count();
    assert_eq!(
        def_count, 1,
        "list_nav_render.rs must define `fn paint_test_result_decoration(` exactly once — duplicate definitions are a sign of accidental copy/paste from the legacy Detail::Summary path (RPC-150). found {def_count}"
    );

    // @step And the file must contain exactly one call site (total `paint_test_result_decoration(` count minus the one `fn` definition equals 1)
    let total = src.matches("paint_test_result_decoration(").count();
    let call_count = total.saturating_sub(def_count);
    assert_eq!(
        call_count, 1,
        "list_nav_render.rs must invoke `paint_test_result_decoration(` exactly once (excluding the `fn` definition) — additional call sites would double-paint the decoration on the same row (RPC-150). found {call_count} (total {total}, defs {def_count})"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: paint_test_result_decoration accepts the canonical
//           six-argument signature
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_paint_test_result_decoration_accepts_the_canonical_six_argument_signature() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    let src = list_nav_render_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the file must contain "fn paint_test_result_decoration("
    assert!(
        src.contains("fn paint_test_result_decoration("),
        "list_nav_render.rs must contain the `fn paint_test_result_decoration(` declaration (RPC-150)"
    );

    // @step And the source must contain each of the six canonical parameter declarations
    for param in [
        "kind: RowKind,",
        "selected: bool,",
        "status: &super::ProviderTestStatus,",
        "row_area: Rect,",
        "end_x: u16,",
        "buf: &mut Buffer,",
    ] {
        assert!(
            src.contains(param),
            "list_nav_render.rs must declare the canonical parameter `{param}` on paint_test_result_decoration — dropping or renaming any parameter silently breaks the call site (RPC-150)"
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: paint_test_result_decoration helper is owned exclusively by
//           list_nav_render.rs
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_paint_test_result_decoration_helper_is_owned_exclusively_by_list_nav_render_rs() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let detail = detail_rs();

    // @step And I read the source of codelet/fspec-tui/src/views/provider_settings/row_render.rs
    let row_render = row_render_rs();

    // @step When I scan each file as a string
    // (nothing to do — we just keep the source strings)

    // @step Then detail.rs must contain zero occurrences of "paint_test_result_decoration"
    let detail_count = detail.matches("paint_test_result_decoration").count();
    assert_eq!(
        detail_count, 0,
        "detail.rs must NOT reference `paint_test_result_decoration` — the Detail::Summary surface was removed in RPC-103; any reappearance here is the RPC-072 stub state silently re-emerging (RPC-150). found {detail_count}"
    );

    // @step And row_render.rs must contain zero occurrences of "paint_test_result_decoration"
    let row_render_count = row_render
        .matches("paint_test_result_decoration")
        .count();
    assert_eq!(
        row_render_count, 0,
        "row_render.rs must NOT reference `paint_test_result_decoration` — row_render is the pure row painter; decoration composition belongs one layer up in the per-row loop (RPC-150). found {row_render_count}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Decoration foreground comes from status decoration and
//           background comes from row_band_bg
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_decoration_foreground_from_status_decoration_and_background_from_row_band_bg() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    let src = list_nav_render_rs();

    // @step When I extract the brace-balanced body of the "fn paint_test_result_decoration" function
    let body = fn_body(&src, "paint_test_result_decoration");

    // @step Then the body must contain "status.decoration()"
    assert!(
        body.contains("status.decoration()"),
        "paint_test_result_decoration body must call `status.decoration()` — the (text, fg) pair is the canonical source of the ladder's appearance; bypassing it would lose the ✓/✗/… glyph + colour contract (RPC-150)"
    );

    // @step And the body must contain "row_band_bg(kind, selected)"
    assert!(
        body.contains("row_band_bg(kind, selected)"),
        "paint_test_result_decoration body must call `row_band_bg(kind, selected)` — the decoration background MUST match the row's underlying band, otherwise the ladder appears as a stripe of the wrong colour and breaks visual continuity (RPC-150)"
    );

    // @step And the body must contain "Style::default().fg(fg).bg(bg)"
    assert!(
        body.contains("Style::default().fg(fg).bg(bg)"),
        "paint_test_result_decoration body must compose the style via `Style::default().fg(fg).bg(bg)` — any other composition risks dropping one of the two channels (RPC-150)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Separator and decoration coordinates respect the row right
//           boundary
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_separator_and_decoration_coordinates_respect_the_row_right_boundary() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    let src = list_nav_render_rs();

    // @step When I extract the brace-balanced body of the "fn paint_test_result_decoration" function
    let body = fn_body(&src, "paint_test_result_decoration");

    // @step Then the body must contain "row_area.x.saturating_add(row_area.width)"
    assert!(
        body.contains("row_area.x.saturating_add(row_area.width)"),
        "paint_test_result_decoration body must compute `right_bound = row_area.x.saturating_add(row_area.width)` — saturating_add is what prevents u16 overflow when row_area lives at the end of the buffer (RPC-150)"
    );

    // @step And the body must contain "if end_x >= right_bound"
    assert!(
        body.contains("if end_x >= right_bound"),
        "paint_test_result_decoration body must guard with `if end_x >= right_bound` (early return) — without this the separator paint at `end_x` could write outside the row area (RPC-150)"
    );

    // @step And the body must contain "let separator_x = end_x;"
    assert!(
        body.contains("let separator_x = end_x;"),
        "paint_test_result_decoration body must declare `let separator_x = end_x;` — the separator is exactly one cell at the row's content terminus; renaming the binding breaks the call-site contract (RPC-150)"
    );

    // @step And the body must contain "let decoration_x = end_x.saturating_add(1);"
    assert!(
        body.contains("let decoration_x = end_x.saturating_add(1);"),
        "paint_test_result_decoration body must declare `let decoration_x = end_x.saturating_add(1);` — saturating_add(1) skips exactly the separator cell so the decoration text never overlaps the row label (RPC-150)"
    );

    // @step And the body must contain "if decoration_x >= right_bound"
    assert!(
        body.contains("if decoration_x >= right_bound"),
        "paint_test_result_decoration body must guard with `if decoration_x >= right_bound` (second early return) — without this a row that is exactly wide enough for the label + separator (but no decoration glyph) would write the ladder outside the row area (RPC-150)"
    );
}
