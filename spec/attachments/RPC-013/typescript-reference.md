# RPC-013 — View-aware footer (Board vs Agent)

## TypeScript reference

### Where the Board's footer lives
`src/tui/components/UnifiedBoardLayout.tsx:504-511`

```tsx
{/* Footer */}
<Box flexDirection="row" height={1}>
  <Text>│</Text>
  <Box flexGrow={1} justifyContent="center">
    <Text>← → Columns ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Work Agent ◆ ESC Back</Text>
  </Box>
  <Text>│</Text>
</Box>
```

The footer is **embedded inside** the UnifiedBoardLayout — it is not a top-level
component. The AgentView footer is a completely separate component
(`src/tui/components/SessionFooter.tsx`) that shows `~/projects/fspec [⌥ codelet-integration]`
on the right and view-specific hints on the left.

## Current Rust state

`codelet/fspec-tui/src/views/footer.rs` renders a single, view-agnostic footer:
```
? help q quit Tab switch pane
```

`codelet/fspec-tui/src/views/navigator.rs:107-117` always paints the same
`FooterView` regardless of `active_view`. This leaks the generic
"help/quit/switch pane" hint into both BoardView and AgentView and is the
regression visible in `rust-unified-board.png` vs the TS screenshot.

## Target Rust behavior

1. **FooterView becomes view-aware.** Either:
   - Option A: `FooterView::render_for(area, buf, view: ViewMode)` matches on the mode and paints the appropriate hint string, OR
   - Option B: BoardView and AgentView each paint their own 1-row footer inside their `render_with_store`, and `Navigator` no longer reserves a footer row — the footer drops out of the navigator and lands inside the views, matching the TS architecture where `UnifiedBoardLayout` owns its footer.

   **Recommended: Option B.** It matches the TS structure 1:1 and keeps the
   Navigator simple (single child view, full area). It also lets each
   view evolve its footer independently (Agent's footer is much richer —
   see RPC-018).

2. **BoardView footer string** (literal port):
   `← → Columns ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Work Agent ◆ ESC Back`

3. **AgentView footer placeholder** for this slice:
   `Enter=send  Ctrl+C=interrupt  ESC=back` — the rich
   `~/projects/fspec [⌥ codelet-integration]` form lands in RPC-018.

## RPC/NAPI boundary

**No new RPC methods required.** Footer strings are static client-side
presentation; the cwd and git branch hints in the AgentView footer arrive in
RPC-018.

## Existing TypeScript behavior preserved

This card touches ONLY `codelet/fspec-tui/src/`. The TS `UnifiedBoardLayout.tsx`
and `SessionFooter.tsx` are unchanged.

## Acceptance criteria sketch

- BoardView renders the literal footer string above when `active_view == Board`.
- AgentView renders the placeholder footer string when `active_view == Agent`.
- The previous `? help q quit Tab switch pane` string no longer appears in
  either view.
- `Tab` is removed from BoardView's footer hint string (it is reserved for the
  in-BoardView panel switch per RPC-012 rule [19], not yet implemented).
