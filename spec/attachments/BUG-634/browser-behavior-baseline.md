# Browser login baseline and regression evidence

The user explicitly requested the behavior from before the TUI-126 rework,
not replacement browser flows. The comparison baseline is `1b19dc9f2^`.

## Preserved behavior

- Anthropic: `claude_browser_oauth_login` opens a local browser page containing
  the Anthropic authorization link and the `code#state` paste form. The user
  authorizes in the browser and submits the returned code through that form.
- Codex: `browser_oauth_login` opens the authorization URL and receives the
  automatic callback at `http://localhost:1455/auth/callback`.
- Separate Anthropic TUI code entry and Codex device authorization remain
  selectable alternatives. They do not replace browser sign-in.
- Cancellation, state validation, and provider-owned credential persistence
  remain part of the flows.

## Confirmed defects

The WebSocket backend inherited unsupported code/device login methods despite
the UI advertising those choices. Browser rows were filtered out because the
backend capability returned false for every daemon connection, including local
daemons. This gate predates TUI-126; the rework did not introduce that gate.

TUI-126 additionally changed URL-focused `c` and `o` to move focus to Code.
Consequently copying then opening, or opening then following the launch-failure
instruction to copy, typed unwanted characters into the code instead.

The first real-WebSocket App regression run failed all four scenarios:
three could not find the Anthropic browser row and code login produced
`headless OAuth login is not supported`. After restoring the Anthropic route,
the explicit Codex browser-row assertion also failed until its capability was
restored. New callback/form regressions preserve the baseline interactions.

## Architecture

Browser and loopback listeners belong to the TUI computer. The connected daemon
exchanges authorization codes and persists credentials. Both embedded and daemon
clients expose wrapped URLs with copy/open actions. This separation avoids
opening a browser on a remote daemon or writing credentials into the client
computer's provider store.

An early implementation automatically opened Anthropic authorization and used
the TUI code field. That design was superseded before completion because the
user explicitly required preserving the browser-hosted paste form.
