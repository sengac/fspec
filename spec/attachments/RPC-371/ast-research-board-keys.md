# AST Research — RPC-371 (umbrella consolidation)

RPC-371 is a tracking umbrella with no implementation of its own. The real
work lives in its three children (RPC-372/373/374). This file consolidates
the AST evidence that the board A/D key handlers were wired end-to-end.

## Board key-handler wiring (`AstGrep` `KeyCode::Char($C)` over `codelet/`)

| Key | Test file (child) | Evidence |
|-----|-------------------|----------|
| A (attachments) | `codelet/fspec-tui/tests/board_open_attachment_rpc374.rs` | `KeyCode::Char('A')` / `'a'` exercised against board view (RPC-374) |
| D (FOUNDATION.md) | `codelet/fspec-tui/tests/board_open_foundation_rpc373.rs` | `KeyCode::Char('D')` / `'d'` exercised against board view (RPC-373) |

The axum viewer server those keys launch is RPC-372
(`codelet/attachment-viewer`), whose public surface (`build_router`,
`ViewerState`, `viewer_template`) is verified in RPC-375's research.

## Findings

- Both A and D keys have dedicated end-to-end board tests — the handlers are
  connected, not orphaned code.
- The supporting axum machinery (RPC-372) is a real workspace crate consumed
  by the board view.
- No umbrella-level code exists or is required; acceptance is the union of the
  three children, all `done`.
