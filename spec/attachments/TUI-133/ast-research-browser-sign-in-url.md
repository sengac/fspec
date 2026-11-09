# AST research: where the browser sign-in URL comes from (TUI-133)

TUI-133 was delivered by BUG-634 (ff7ac6c8a). The URL the waiting screen shows comes from the client-hosted listeners:

## fspec astgrep --pattern "pub fn browser_url(&self) -> &str { $$$ }" --lang rust --path rust/providers/src
```
rust/providers/src/claude_browser_authorization.rs:63:5:pub fn browser_url(&self) -> &str {

```

## fspec astgrep --pattern "Action::OAuthBrowserReady { $$$ }" --lang rust --path rust/fspec-tui/src/app
```
rust/fspec-tui/src/app/oauth_browser.rs:24:32:Action::OAuthBrowserReady {
rust/fspec-tui/src/app/oauth_claude_browser.rs:18:32:Action::OAuthBrowserReady {

```

## fspec astgrep --pattern "pub(super) fn render_browser_waiting($$$A) { $$$B }" --lang rust --path rust/fspec-tui/src/views
```
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:82:1:pub(super) fn render_browser_waiting(
```

render_browser_waiting shows the URL under "Visit:" wrapped, with "c: copy URL · o: open URL"; copy and open are handled in views/provider_settings/oauth_login.rs (OAuthBrowserWaiting arm).
