# RPC-392 — Full-Width Background Padding for Colored Edit/Write Diff Lines

## Summary

In the Rust `fspec-tui` agent view, decoded `[R]-` / `[A]+` diff lines color **only the
literal text characters**. The dark-red / dark-green background therefore stops at the
end of the line's content instead of filling the row out to the viewport width. In the
TypeScript reference the colored block fills the **entire row width** edge-to-edge,
producing solid colored bars. This bug closes that visual-parity gap.

This is a **rendering/layout** bug only. The diff *generation* (RPC-390) and the
*marker decode* (RPC-391) are correct. We change how the decoded removed/added lines
are padded before the background style is applied.

---

## The TypeScript Reference Behaviour (source of truth)

File: `src/tui/components/AgentView.tsx`, `VirtualList renderItem` (≈ lines 5345-5391).

```tsx
// Changed line with [R] or [A] marker - entire line gets colored background
if (rIdx >= 0 || aIdx >= 0) {
  const markerIdx = rIdx >= 0 ? rIdx : aIdx;
  const markerType = rIdx >= 0 ? 'R' : 'A';
  const lineWithoutMarker =
    content.slice(0, markerIdx) + content.slice(markerIdx + 3);

  return (
    <Box flexGrow={1}>                         {/* ← fills the row width */}
      <Text
        backgroundColor={
          markerType === 'R' ? DIFF_COLORS.removed : DIFF_COLORS.added
        }
        color="white"
      >
        {lineWithoutMarker}
      </Text>
    </Box>
  );
}
```

`<Box flexGrow={1}>` makes the parent flex container grow to the full available width.
Because the `<Text backgroundColor=…>` lives inside it and Ink paints the background of
its box, the colored block visually extends to the **right edge of the panel**, regardless
of how short the line content is. That is the "padding around the edited lines" the user
is describing.

The DIFF colours (TS `DIFF_COLORS`, AgentView.tsx ≈ 608-611) are:

| Kind    | Hex       | RGB           |
|---------|-----------|---------------|
| removed | `#8B0000` | `139, 0, 0`   |
| added   | `#006400` | `0, 100, 0`   |

White foreground (`color="white"`) on both.

### Context lines and other lines DO NOT get the full-width treatment

In the same `renderItem`, **context** lines (gray gutter + white content) and plain lines
are rendered with normal `<Text>` and `<Box flexGrow={1}>` wrappers but **no
`backgroundColor`**. They have no visible background, so right-padding them would be a
no-op visually. We must NOT add a colored background to them. Only the `[R]`/`[A]` lines
get the solid bar.

---

## The Rust Bug (current behaviour)

File: `codelet/fspec-tui/src/store/agent_view/diff_decode.rs`

```rust
fn colored_span(text: String, bg: Color) -> Span<'static> {
    Span::styled(text, Style::default().bg(bg).fg(Color::White))
}

pub fn decode_diff_line(line: &str) -> Vec<Span<'static>> {
    if let Some(idx) = line.find("[R]") {
        return vec![colored_span(strip_marker(line, idx), DIFF_BG_REMOVED)];
    }
    if let Some(idx) = line.find("[A]") {
        return vec![colored_span(strip_marker(line, idx), DIFF_BG_ADDED)];
    }
    // … context gutter / raw …
}
```

The `Span` covers only `strip_marker(line, idx)` — i.e. exactly the text characters.
ratatui paints a span's background only under its own cells, so the background ends where
the text ends. There is **no trailing pad to the row width**, hence no full-width bar.

### Call sites that must produce the padded bar

1. **Scrollback** — `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs`
   (the `ChunkKind::ToolCall { is_diff: true, .. }` branch, ≈ lines 136-146):
   ```rust
   for hard in body.split('\n') {
       for w in wrap_diff_line(hard, width) {
           if is_decoded_diff_line(&w) {
               out.push(Line::from(decode_diff_line(&w)));
           } else {
               out.push(Line::from(Span::styled(w, body_style)));
           }
       }
   }
   ```
   Here the available row width is `width` (the viewport width passed into `wrap_source`).

2. **Turn-content modal** — `codelet/fspec-tui/src/views/agent/turn_modal.rs`
   (≈ line 116) via `decode_modal_row`:
   ```rust
   .map(|w| DialogRow {
       spans: crate::store::agent_view::diff_decode::decode_modal_row(&w),
       …
   })
   ```
   Here the row width is `content_width` (`geom.content_width`).

Because the available width differs per call site (and the modal vs scrollback differ),
the **width must be passed in** to the decode function rather than hard-coded.

---

## Required Behaviour (acceptance criteria)

1. A decoded `[R]-` line, after marker stripping, is **right-padded with spaces** to the
   target render width and the dark-red background + white fg covers the entire padded
   width (a solid bar to the right edge).
2. A decoded `[A]+` line behaves the same with the dark-green background.
3. **Context** lines (gray gutter + white content) are NOT padded with a colored
   background — they render exactly as before (gray gutter span + white content span, no
   background fill).
4. **Gap markers** (`… (N lines)`) and the `... +N lines (select turn to /expand)`
   indicator, and any plain/non-diff line, render exactly as before (no colored bar).
5. The full-width bar applies in **both** the scrollback (`chunk_wrap.rs`) and the
   turn-content modal (`turn_modal.rs` / `decode_modal_row`).
6. If the (marker-stripped) content is already **as wide as or wider than** the target
   width, no padding is added and nothing is truncated beyond the existing wrap behaviour
   (the line was already wrapped to width upstream, so the common case is content ≤ width).
7. Non-Edit/Write tool output (Bash, Grep, etc.) is unaffected — no regression.
8. A zero/very-small width must not panic and must not produce negative pad counts
   (saturating arithmetic).

---

## Implementation Plan (guidance, not prescriptive line numbers)

### A. Add a width-aware decode in `diff_decode.rs`

Introduce a width parameter on the decode path. Recommended shape (keeps the existing
zero-arg API working for tests that don't care about width, OR migrate call sites):

```rust
/// Decode a marker-encoded diff line, padding [R]/[A] bars to `width` columns
/// so the background fills the row (parity with the TS `<Box flexGrow={1}>`).
pub fn decode_diff_line_padded(line: &str, width: usize) -> Vec<Span<'static>> {
    if let Some(idx) = line.find("[R]") {
        return vec![colored_span(pad_to_width(strip_marker(line, idx), width), DIFF_BG_REMOVED)];
    }
    if let Some(idx) = line.find("[A]") {
        return vec![colored_span(pad_to_width(strip_marker(line, idx), width), DIFF_BG_ADDED)];
    }
    // context gutter + plain: unchanged (NO padding / NO background)
    …
}

fn pad_to_width(text: String, width: usize) -> String {
    let display = unicode_display_width(&text); // count display columns, not bytes
    if display >= width {
        text
    } else {
        let mut out = text;
        out.extend(std::iter::repeat(' ').take(width - display));
        out
    }
}
```

- Use **display width** (the same width metric `wrap_to_width` uses), not `.len()` bytes
  or `.chars().count()`, so wide/!ASCII chars pad correctly. Check
  `codelet/fspec-tui/src/views/agent/text_wrap.rs` for the existing width helper and reuse
  it (DRY — do NOT roll a second width function).
- `colored_span`, `strip_marker`, `context_gutter_len` stay as-is for the context/plain
  paths.

### B. Wire the width through the call sites

- `chunk_wrap.rs` diff branch: call `decode_diff_line_padded(&w, width as usize)` instead
  of `decode_diff_line(&w)`.
- `decode_modal_row` (modal): give it a width param (`decode_modal_row(row, width)`), and
  in `turn_modal.rs` pass `content_width`. Only diff lines get padded; non-diff modal rows
  stay a single raw span.

### C. Keep the no-width API or update all callers

Either keep `decode_diff_line` as a thin wrapper (`decode_diff_line_padded(line, 0)` → no
pad) for existing unit tests, or update the existing tests to the new signature. Prefer
the smallest, clearest change; do not leave a dead/duplicated function (DRY).

---

## Test Plan

Unit tests live alongside the modules (Rust `#[cfg(test)]`), plus the feature-linked
integration test file. Every Gherkin step gets a matching `@step` comment.

### Scenarios → tests

1. **Removed line is padded to a full-width red bar**
   - Given a decoded `[R]-` line shorter than the render width
   - When it is decoded with that width
   - Then the resulting span's content display-width equals the render width
   - And the span background is `#8B0000` and fg is white
   - And the content contains no `[R]` marker.

2. **Added line is padded to a full-width green bar**
   - Same as above with `[A]` / `#006400`.

3. **Context line is not given a colored background bar**
   - Given a decoded context line (`L 250   foo`)
   - When it is decoded with a render width wider than the line
   - Then it produces a gray-gutter span + white-content span
   - And neither span has a background colour
   - And the total content is NOT padded to the render width with a background.

4. **Gap-marker / plain line is unchanged**
   - Given a `... (5 lines)` gap-marker (or `... +N lines …` indicator)
   - When decoded with a render width
   - Then it is a single span with no background and no extra padding bar.

5. **Content already at/over width is not padded or truncated**
   - Given a decoded `[A]+` line whose stripped content display-width ≥ render width
   - When decoded with that width
   - Then the content is returned unchanged (no added spaces, no truncation), still
     colored.

6. **Zero / tiny width does not panic and pads non-negatively**
   - Given a decoded `[R]-` line
   - When decoded with width 0
   - Then it does not panic and adds no negative padding (content returned as-is).

7. **Scrollback diff branch emits full-width bars** (integration, `chunk_wrap`)
   - Given a `ChunkKind::ToolCall { is_diff: true }` whose body has a removed and an added
     line
   - When the source is wrapped at a known width
   - Then the removed/added `Line`s each carry a span whose content display-width equals
     the wrap width and the correct background.

8. **Modal diff rows emit full-width bars; non-diff rows do not** (integration, modal)
   - Given a modal body containing a `[R]`/`[A]` row and a plain row
   - When the modal decodes rows at `content_width`
   - Then the diff rows are padded full-width with the diff background and the plain row is
     a single unpadded raw span.

---

## Files In Scope

- `codelet/fspec-tui/src/store/agent_view/diff_decode.rs` (add width-aware pad path)
- `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` (pass `width` in the diff branch)
- `codelet/fspec-tui/src/views/agent/turn_modal.rs` (pass `content_width` to modal decode)
- `codelet/fspec-tui/src/views/agent/text_wrap.rs` (reuse the existing display-width
  helper — do not duplicate)
- Feature file: `spec/features/agentview-edit-diff-padding.feature` (new, `@RPC-392`)
- Tests: Rust `#[cfg(test)]` in the touched modules + a feature-linked integration test

## Out of Scope

- Diff generation / marker encoding (RPC-390) — unchanged.
- Context-window collapse logic (RPC-389) — unchanged.
- Any TypeScript code — this is Rust-only parity work.

## Constraints / Standards

- Rust: no `unwrap()` / `todo!()` / `unimplemented!()` in production paths; saturating
  arithmetic for the pad count; files stay < 300 LoC; `cargo clippy` clean;
  `cargo fmt --check` clean.
- Reuse the existing display-width function (DRY) — count display columns, not bytes.
- 100% scenario coverage with accurate `link-coverage` line ranges and `@step` comments.
