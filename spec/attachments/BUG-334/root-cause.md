# BUG-334 root cause: the logout-failure step was neither tested nor true

## Symptom

`spec/features/provider-settings-oauth-disconnect.feature`, scenario "A backend clear error returns
to list with a safe failure message", step `And a clear logout failure is shown without credential
contents` had no `@step` in `rust/fspec-tui/tests/prov112/dispatch.rs`
(`backend_clear_error_returns_to_list_without_leaking_rpc_name`, lines 124-201 before the fix).
The test only checked that `provider_settings.status` did not contain RPC names; it never checked
that a failure was shown.

## Evidence that the behaviour was also missing

- `rust/fspec-tui/src/app/oauth_disconnect.rs:13-18` — on `oauth_clear_tokens` error the dispatch
  logs it and sends `Action::ProviderSettingsStatus("✗ OAuth logout failed; credentials may
  remain. Retry after checking storage access.")`, which `handle_provider_settings_status`
  (`dispatch_provider_settings.rs:220-222`) stores in `ProviderSettingsView::status`.
- The confirm returns the view to `ProviderSettingsMode::List`
  (`views/provider_settings/oauth_confirm.rs:40`).
- `views/provider_settings/body_render.rs:24` renders List mode with `list::render_list`, and
  `list.rs:202-286` never paints `view.status`. Only the form and Detail renderers do
  (`body_render.rs:29,32,40`, `detail.rs:256-262`).

So the failure text existed only in memory: a user whose logout failed saw the list come back with
no message. Writing the missing step as a real assertion against the rendered view makes the test
fail (red) for that reason.

## Fix

`render_mode_body`'s List arm reserves the bottom body row for `view.status` when it is non-empty
and paints it there (red when it starts with `✗`, otherwise default colour); the list renders in the
remaining rows. The status message itself is a fixed sentence and never includes the backend error.
This also makes the other List-mode results that already set `status` (credential deletion,
default-model errors) visible.

The test now scripts a backend error carrying a credential-shaped token, renders the Provider
Settings view into a buffer and asserts that the failure sentence is visible while neither the token
nor any RPC/method name is.

## Alternatives rejected

- **Assert only on `provider_settings.status`** — proves state, not that the user sees it (the
  original gap).
- **Paint the status inside `list::render_list`** — `list.rs` is 286 lines and would cross the
  300-line limit; `body_render.rs` (97 lines) already owns per-mode body layout.
- **Show the backend error text** — would leak credential contents and method names.

## Wire / API changes

None.
