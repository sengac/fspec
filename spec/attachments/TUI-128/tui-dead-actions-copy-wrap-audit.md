# TUI audit: dead actions, advertised keys that don't work, copy gaps, wrap gaps, UI stubs

Card: TUI-126, plus the new cards proposed in section 2.
Repo: `~/projects/fspec/rust`. Paths are relative to `rust/fspec-tui/src/` unless stated otherwise.
Method: read-only. Nothing was edited, built or run, and no state-changing git command was used. Line numbers are from 2026-11-07.

Scope covered:
- every `Action` variant (281, composed in `components/mod.rs:115-151` from `components/action_*.rs`), checked against `app/dispatch.rs`, the `try_dispatch_*` chain (`app/dispatch_capability.rs:17-42`), `Navigator::apply_action`, `Compositor::update` and the view-event translators;
- about 55 hint and footer surfaces, plus every Provider Settings mode;
- every dialog and full-screen view, for copy and wrap;
- `rust/tui` and `rust/cli/src/interactive`. These have no action bus and no key handling (the CLI writes plain stdout), so nothing in them applies. The `rust/tui` `StatusDisplay`, `create_event_stream`, `restore_terminal` and `set_panic_hook` have no callers outside the crate (ST-4).

The high-severity claims were re-checked by hand against the source:
- DA-5: `MergeConfirmDialog::new` at `app/dispatch_merge_worktree.rs:107` is called without `with_action_tx`.
- AK-1: the compositor returns ignored, then Stage 3 runs (`app/events.rs:80-117`), and `ErrorDialog` returns `ignored()` for non-Esc keys (`components/error_dialog.rs:79`).
- ST-1: there are no WebSocket `oauth_*` overrides; the trait defaults are in `transport/backend/credentials.rs:57-130`.
- AK-4: End is mapped to `ScrollbackPageDown` (`views/agent/dispatch.rs:211`).
- The Provider Settings findings were all read directly.

---

## 1. Totals

| Group | Count | IDs |
|---|---|---|
| Provider Settings OAuth (TUI-126 core) | 15 | PS-1 … PS-15 (they absorb DA-1…DA-4) |
| 1. Dead actions / broken emit paths | 5 + cleanup | DA-5, DA-6, DA-7(=ST-1), DA-8, DA-9 (cleanup list) |
| 2. Advertised keys that don't work | 10 | AK-1 … AK-10 |
| 3. Copy gaps | 14 + agent Ctrl+C | CP-1 … CP-14, CP-15 |
| 4. Wrap and truncation gaps | 11 + board footer | WR-1 … WR-11, WR-12 |
| 5. UI wired to a stub | 4 | ST-1 … ST-4 (AK-2 is also a stub) |

Highest severity:
1. **PS-1, PS-2:** `c: copy URL` and `o: open URL` in Anthropic "Sign in with code" only log. This is the product owner's report.
2. **PS-4:** pasting the code into "Code:" is silently dropped, so the user has to type a code of about 90 characters by hand.
3. **PS-6:** the authorize URL is about 384 characters and is painted on one unwrapped row. At 80 columns 73 characters are visible, and at 120 columns 113. It can't be read, and Shift+drag can't copy it either.
4. **DA-5:** the `/merge-worktree` dialog's Merge and Discard buttons close the dialog and do nothing, because the dialog is built without an action sender.
5. **AK-1:** modal dialogs, including the API Error dialog, pass unhandled keys to the view behind them. Typing "ok" then Enter under an API Error dialog sends a chat message.
6. **ST-1 / DA-7:** on the WebSocket transport (client mode, daemon switch), every OAuth call is a stub, but the "Sign in with code", device and Logout rows are still shown.
7. **AK-2:** the blocklist "Toggle Rule" only changes the display. The rule stays enforced.
8. **CP-1, CP-2, WR-1:** error dialogs can't be copied, and the API Error dialog is clipped (76 visible characters at 80 columns).

---

## 2. Proposed cards

| Card | Title | Findings | Priority |
|---|---|---|---|
| **TUI-126** (existing) | Provider Settings OAuth screens: copy, open, paste and wrap work | PS-1 … PS-15 | P0 |
| NEW-A | Merge dialog and other emits that never reach a handler | DA-5, DA-6, DA-8 | P0 (DA-5) |
| NEW-B | Modal dialogs swallow every key (no pass-through to the hidden view) | AK-1 | P0 |
| NEW-C | Advertised keys match their bindings | AK-3 … AK-10 | P2 |
| NEW-D | Copy everywhere text is shown, with one copy path and a confirmation | CP-1 … CP-15 | P1 |
| NEW-E | Wrap long text in dialogs, panes and footers | WR-1 … WR-12 | P1 |
| NEW-F | OAuth on the WebSocket transport: forward the RPCs or hide the rows | ST-1 (= DA-7) | P1 |
| NEW-G | Blocklist toggle reaches enforcement, or is relabelled view-only | AK-2 | P1 |
| NEW-H | Cleanup: vestigial actions and scaffolding | DA-9, ST-2, ST-3, ST-4, PS-14, PS-15 | P3 |

Suggested order: TUI-126, then NEW-A and NEW-B (small and high impact), then NEW-D and NEW-E together, because they share the wrapped-rows helper and the copy key in each dialog.

---

## 3. Shared machinery to reuse (for every fixer)

**Copy path.** A view emits `Action::CopyToClipboard(String)` (declared at `components/action_transcript_selection.rs:52`):
- That reaches `app/dispatch.rs:231`, then `App::handle_copy_to_clipboard` (`app/dispatch_scroll.rs:234-241`), then `Osc52Clipboard::copy` (`mouse/clipboard.rs`), which writes `ESC]52;c;<base64>BEL`.
- Empty text is a no-op. A failure is only logged. There is no user feedback.
- Today's emitters: composer selection (`views/agent/mouse_dispatch.rs:279`), turn-modal selection (`views/agent/turn_modal_select.rs:184`), Provider Settings Ctrl+C (`views/provider_settings/copy.rs:81`).

**Other copy paths:**
- Agent scrollback selection uses `Selection*` actions and writes `self.clipboard` directly (`dispatch_scroll.rs:190-229`, via `views/agent/scrollback_copy.rs:30`).
- The board details strip has its **own** writer (`views/board.rs:107`), so `App::set_clipboard_writer_for_test` does not see board copies.
- Drag selection exists only in the agent scrollback, the composer, the turn modal and the board strip.
- Mouse capture is always on (`terminal/mod.rs:103`), and the release-capture toggle is unwired (ST-2). Every other view and dialog has no way to copy except terminal Shift+drag, which copies only visible cells.

**Provider Settings Ctrl+C:**
- `views/provider_settings/copy.rs:26` `intercept_ctrl_c` is gated by `is_copyable_mode` (`copy.rs:43`), which covers only the profile forms and EditApiKey.
- `handle_copy` (`copy.rs:59`) masks secrets with `mask_secret`.
- After that, `views/provider_settings/mod.rs:196-201` consumes **every** Ctrl or Alt key.

**URL opener:** `app/board_card_attachments.rs:27`:
- `AttachmentLauncher = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>`, with production `open::that` (`:80`) run via `spawn_blocking` (`:223`).
- Test seam: `App::set_board_attachment_launcher_for_test` (`:159`).
- Lift it to an App-level `url_launcher` for PS-2.

**Wrap helpers:**
- `views/agent/text_wrap.rs:29` `wrap_to_width(text, width)`: display-width aware, honours `\n`, and slices long tokens such as URLs.
- `components/board_card/wrap.rs:7` `wrap`.
- `components/daemon_form_dialog.rs:46` `scroll_window` (one-line horizontal input scroll).
- The fill, wrap and scroll dialog pattern in `components/session_storage_dialog.rs:97-126`.
- For full-screen bodies, `Paragraph::new(lines).wrap(Wrap { trim: false })`. None of the Provider Settings renderers use it today.

**Test harness** (rust/fspec-tui/tests):
- `common::MockBackend`, including `seed_oauth_headless_start(url, verifier)` and `seed_oauth_device_start(..)` (`common/mock_scripting_5.rs:12,35`).
- `App::new(Arc<dyn FspecBackend>)`, `App::handle_event(&Event)`, `App::handle_paste(&str)` (`app/composer_draft.rs:97,105`), `App::try_recv_action()` and `App::next_pending_task()`.
- `App::set_clipboard_writer_for_test(Box<dyn Write+Send>)`.
- File-local helpers in `tests/provider_settings_input_copy_prov138.rs`: `SharedWriter` (:109), `app_in_provider_view` (:128), `drain` (:142), `clip_bytes` (:148), `osc52` (:153).
- File-local helpers in `tests/provider_settings_oauth_login_prov113.rs`: `render_screen` / `screen_contains` / `row_string` (:126-149), `drain_pending` (:556), `wait_until` (:569), `fresh_app` (:586), `provider_settings_mode` (:591).
- Lift `SharedWriter` and `osc52` into `tests/common` so every new copy test can share them.
- Agent and board tests: `common::harness::AppTestHarness::new()` / `::empty()` (`press_key`, `key`, `key_mod`, `compositor_contains`, `active_view`, `input_value`). Use `test_app` + `render_one_frame` + `buffer_to_rows` (80x24 TestBackend), or `common/text_selection_anchor_copy010_helpers.rs:76` `render_app(app, w, h)` for any size.

**Why the existing test missed TUI-126.** `tests/provider_settings_oauth_login_prov113.rs:487` (`headless_code_entry_c_copies_only_while_empty`) only checks that the **view emits** `OAuthCopyUrl`. It never dispatches the action through `App`, so it never checks the clipboard bytes. The same pattern hid DA-5: the merge tests read `take_pending_outcome` directly. Every test below drives **App** from the key event to the side effect.

---

## 4. TUI-126 core: Provider Settings OAuth screens (PS-1 … PS-15)

Mode → key handler / renderer map:
- `views/provider_settings/mod.rs:188-233` `handle_key` (the OAuth modes route to `oauth_login.rs:49`, and the copilot preamble to `oauth_copilot.rs:52`).
- `body_render.rs:24-104` (render).
- `footer_hints.rs:366-393` (footer).
- `paste.rs:17-36` (paste).
- `copy.rs` (Ctrl+C).
- App handlers: `app/dispatch_provider_settings_oauth.rs:18-87` and `app/dispatch_provider_settings_copilot.rs`.
- Body area = the full terminal width × (height − 3) (`views/full_screen_shell.rs:22,105-115`). There are no side borders.

### PS-1: `c: copy URL` does nothing (OAuthCopyUrl only logs). HIGH. The product owner's bug (= DA-1).
- **Emitter.** `views/provider_settings/oauth_login.rs:159-163`. `KeyCode::Char('c')` while `code_input.is_empty()` emits `Action::OAuthCopyUrl { url }`.
- **Routing.** `app/dispatch_provider_settings.rs:284-286` sends it to `try_dispatch_oauth`.
- **Handler.** `app/dispatch_provider_settings_oauth.rs:78-80`: `tracing::info!(url = %url, "oauth copy-url requested")` only.
- **Hints.**
  - Body line `oauth_login_render.rs:105`: "Enter: submit · c: copy URL · o: open URL · Esc: cancel".
  - Footer `footer_hints.rs:39-40`: "Enter: submit · c: copy · o: open · Esc: cancel".
- **What the user sees.** Nothing. There is no clipboard write and no feedback.
- **Fix.**
  - In the arm, call `self.handle_copy_to_clipboard(url.clone())`, then set a confirmation (see PS-13).
  - Or delete `OAuthCopyUrl` and emit `Action::CopyToClipboard(authorize_url)` straight from the view, as `copy.rs:81` does. If you do that, update the PROV-113 test.
- **Test.**
  1. `mock.seed_oauth_headless_start(URL_384, "v")`, then `fresh_app(mock)`, then `set_clipboard_writer_for_test(SharedWriter(buf))`.
  2. Dispatch `OpenProviderSettingsView`, then `OAuthHeadlessReady { provider_id: "anthropic", authorize_url: URL_384, pkce_verifier: "v", generation: app.navigator().provider_settings.oauth_generation() }`.
  3. `handle_event(Key 'c')`, then `drain`.
  4. Assert `clip_bytes(&buf) == osc52(URL_384)`, which is the full URL and not the visible part.

### PS-2: `o: open URL` does nothing (OAuthOpenUrl only logs). HIGH (= DA-2).
- **Emitter.** `oauth_login.rs:164-168`.
- **Handler.** `dispatch_provider_settings_oauth.rs:75-77`, which is `tracing::info!` only.
- **Hints.** The same two hints as PS-1.
- **What the user sees.** No browser opens, and there is no feedback.
- **Fix.**
  - Reuse the `AttachmentLauncher` seam (section 3): `spawn_blocking(move || launcher(&url))`.
  - On `Err`, show "Could not open a browser. Press c to copy the URL." in the code-entry screen (PS-13).
- **Test.** `set_board_attachment_launcher_for_test(recorder)`, or the new App-level seam. Enter code-entry as in PS-1, press `o`, then `drain` and `next_pending_task().await`. Assert that the recorder got `[URL_384]`. Second case: a launcher that returns `Err`, then assert the status text in `render_screen`.

### PS-3: Ctrl+C is swallowed in every OAuth mode. MEDIUM (= DA-3)
- **Consumer.** `mod.rs:193-201`. `copy::intercept_ctrl_c` runs first, but `is_copyable_mode` (`copy.rs:43`) excludes `OAuthHeadlessCodeEntry`, `OAuthDeviceWaiting`, `OAuthError`, `OAuthEnterpriseUrlEntry` and `OAuthBrowserWaiting`. The blanket `CONTROL|ALT => Consumed` (`mod.rs:196-201`) then eats the key.
- **What the user sees.** The copy wrapper doesn't work on the authorize URL, the device code, the verification URL or the error text.
- **Fix.** Extend `is_copyable_mode` and `handle_copy` (`copy.rs:59`). None of these values is a secret, so don't mask them:
  - code entry: the authorize URL (or `code_input` once it is non-empty);
  - device waiting: the user code (or `"{code}  {url}"`);
  - error: the `error` text;
  - enterprise entry: `url_input`.
- **Test.** Use the `app_in_provider_view` pattern. Set each mode (directly through `navigator_mut().provider_settings.mode`, or through the OAuth `*Ready` / `OAuthLoginFailed` actions), press `ctrl_c()`, `drain`, and assert `osc52(expected)`.

### PS-4: Paste is dropped in code entry and in the GitHub Enterprise host entry. HIGH (= DA-4)
- **Consumer.** `views/provider_settings/paste.rs:17-36`. Only the profile forms and `EditApiKey` take paste. Everything else falls to `_ => ProviderSettingsEvent::Ignored` (`paste.rs:35`).
- **Translator.** `views/navigator_events.rs:38-41` turns that into `EventResult::ignored()`, and `App::route_paste` (`app/events.rs:219-247`) drops it.
- **What the user sees.**
  - The user copies `code#state` (about 90 characters) from console.anthropic.com, pastes into "Code:", and nothing appears.
  - Typing it by hand runs into PS-5.
  - Paste into the enterprise "URL or domain:" field is also dropped.
- **Fix.**
  - Add a paste arm for `OAuthHeadlessCodeEntry`: append `text.trim()` without newlines, then rebuild the mode as `oauth_login.rs:181-186` does.
  - Add a paste arm for `OAuthEnterpriseUrlEntry`: filter through the same `' '..='~'` gate as `oauth_copilot.rs:142`, and clear `validation_error`.
- **Test.**
  1. Enter code-entry as in PS-1.
  2. `app.handle_paste("abc#xyz\n")`.
  3. Assert that `provider_settings_mode(&app)` has `code_input == "abc#xyz"`.
  4. `handle_event(Enter)`, then `try_recv_action()`. Expect `OAuthLoginHeadlessSubmit { code: "abc#xyz", .. }`.

### PS-5: A typed code that starts with `c` or `o` loses its first character. LOW-MEDIUM
- **Where.** `oauth_login.rs:159,164`. With `code_input` empty, `c` and `o` are hotkeys rather than text.
- **What the user sees.** Anthropic codes are random base64url and can start with `c` or `o`. If the user types the code by hand (the only option until PS-4 is fixed), the first `c` copies the URL, or the first `o` (once PS-2 is fixed) opens a browser. That character never reaches the input. The footer also keeps saying "c: copy · o: open" after typing starts, when those keys are now literal.
- **Fix.**
  - Once PS-4 lands, paste covers the normal path.
  - Also, make the footer and body hint depend on the state: when `code_input` is non-empty, show "Enter: submit · Backspace · Esc: cancel".
  - Optionally move copy and open to keys that can't appear in a code, such as Ctrl+C (PS-3) and Ctrl+O. `mod.rs:196` already intercepts Ctrl, so those would have to be checked before that guard.
- **Test.** Enter code-entry, type `"x"`, render with `render_screen`, and assert that `screen_contains("c: copy")` is false. Separately, assert that pressing `c` first leaves `code_input` empty, which documents the trade-off.

### PS-6: The authorize URL is painted on one unwrapped row, so it can't be read or Shift+drag copied. HIGH
- **Render.** `oauth_login_render.rs:89-110` (`render_headless_code_entry`): `Line::from(format!("Visit: {authorize_url}"))` (:101) in a `Paragraph` with no `.wrap(..)`.
- **Length.** `providers/src/claude_oauth.rs:77-96` builds `https://claude.ai/oauth/authorize?code=true&client_id=<36>&response_type=code&redirect_uri=<64 encoded>&scope=<54 encoded>&code_challenge=<43>&code_challenge_method=S256&state=<43>`, which is **about 384 characters**, or about 391 with `Visit: `.
- **Where it breaks.**

  | Terminal width | URL characters visible |
  |---|---|
  | 80 | 73 (19%) |
  | 120 | 113 |
  | 200 | 193 |

  The whole URL fits only at **about 392 columns or more**.
- **What the user sees.** A URL cut off at the right edge, and the browser fails if they retype what is visible.
- **Fix.**
  - `Paragraph::new(lines).wrap(Wrap { trim: false })`. ratatui's word wrap breaks a long token at the width.
  - Or pre-wrap the URL with `text_wrap::wrap_to_width(url, area.width)` and push one `Line` per chunk, so a row holds only URL characters and a Shift+drag of those rows is clean.
  - The 6-line body grows to about 10 rows at 80 columns, which fits in a 24-row terminal (21 body rows).
- **Test.** `render_screen` at `Rect::new(0,0,80,24)` and at 120x30, with the real 384-character URL. Join `row_string(y)` across the rows below "Visit:" with the trailing padding trimmed, and assert that the joined text contains the whole URL.

### PS-7: The typed or pasted code line is clipped. LOW-MEDIUM
- **Render.** `oauth_login_render.rs:103`: `"Code: {code_input}"` on one row. There is no cursor, and no horizontal scroll.
- **What the user sees.** A code of about 90 characters plus `Code: ` (96 in total) is clipped at 80 columns, so the user can't see the end of what they typed or pasted.
- **Fix.** Wrap it, as in PS-6, or use `daemon_form_dialog::scroll_window` so the tail stays visible. Add a cursor block as the EditApiKey render does (`detail.rs` `"█"`).
- **Test.** Paste a 90-character code, render at 80 columns, and assert that the last 10 characters of the code are on screen.

### PS-8: The device-waiting screen has no copy or open key for the user code and verification URL. MEDIUM
- **Render.** `oauth_login_render.rs:68-87`: "Your code: {user_code}" (:81) and "Visit: {verification_url}" (:82).
- **Keys.** `oauth_login.rs:55-59` handles only Esc. Everything else is `Consumed`.
- **Footer.** `footer_hints.rs` gives "Esc: cancel".
- **Used by.** Codex headless (`dispatch_provider_settings_oauth.rs:118-137`, `OAuthDeviceReady`) and GitHub Copilot (`oauth_copilot.rs:163-180`).
- **What the user sees.** The user has to read and retype the code, then open the URL by hand. Ctrl+C is swallowed (PS-3).
- **Fix.** Reuse PS-1 and PS-2: `c` copies `user_code` and `o` opens `verification_url`. No text is typed in this mode, so these keys don't conflict with anything. Advertise them as "c: copy code · o: open URL · Esc: cancel".
- **Test.** `mock.seed_oauth_device_start(..)`, then dispatch `OAuthDeviceReady {..}` with the current generation. Press `c` and assert `osc52(user_code)`. Press `o` and assert the launcher recorded `verification_url`.

### PS-9: The Copilot device screen shows an empty code and URL while the start is pending. LOW
- **Where.** `oauth_copilot.rs:171-175` sets `OAuthDeviceWaiting { user_code: "", verification_url: "" }` before the start resolves. The renderer paints "Your code: " and "Visit: " with nothing after them (`oauth_login_render.rs:81-82`).
- **Fix.** When the fields are empty, render "Requesting device code…" instead. Once PS-8 lands, make `c` and `o` no-ops while the fields are empty.
- **Test.** Press Enter on the GitHub.com option, `render_screen`, and assert "Requesting device code" is shown and "Your code: " with an empty value is not.

### PS-10: The browser-login waiting screen never shows the URL. If the browser can't open, the user is stuck. MEDIUM
- **Render.** `oauth_login_render.rs:55-66` shows only "Waiting for authorization..." and "Press Esc to cancel".
- **Backend.**
  - `providers/src/claude_oauth_server.rs:118-121`: when `open::that` fails, the URL `http://localhost:{port}/` is written **only to tracing** ("Please open this URL manually").
  - `providers/src/codex/codex_oauth_server.rs:87-90` does the same for the Codex authorize URL.
  - The TUI owns the terminal, so the user never sees that log line.
- **What the user sees.** No browser opens (for example on a headless Linux box or over SSH with the embedded transport), there is no URL to visit, and the screen waits until timeout or Esc.
- **Fix.** This needs a backend surface:
  - Return the local form URL or authorize URL from a `oauth_browser_login_start`, or push it as an `OAuthBrowserUrl { url, generation }` action.
  - Render it wrapped (PS-6) with the same `c` and `o` keys (PS-1, PS-2).
  - Lower-effort option: in the waiting screen, add "Browser didn't open? Esc, then use Sign in with code".
- **Test.** With the new action: dispatch `OAuthBrowserUrl{..}` with the generation, render at 80 columns, assert that the whole URL is present across the wrapped rows, press `c`, and assert `osc52(url)`.

### PS-11: The OAuth error message is one clipped row and can't be copied. MEDIUM
- **Render.** `oauth_login_render.rs:126-140`: `Line::from(error.to_string())` (:133), with no wrap.
- **Source.** `dispatch_provider_settings_oauth.rs:274-280` (`fail_action`, `error.to_string()` of an anyhow chain). Token-exchange failures carry HTTP status and body text that easily goes past 80 characters.
- **What the user sees.** A cut-off error, with no copy (PS-3).
- **Fix.** `.wrap(Wrap { trim: false })`. With PS-3 in place, Ctrl+C or `c` copies `error`. Since only Enter and Esc are bound in this mode, adding `c` is safe. Advertise it as "Enter: retry · c: copy error · Esc: back".
- **Test.** Dispatch `OAuthLoginFailed { error: 250 chars, generation }`, render at 80x24, and assert the error's last word is on screen. Press `c` and assert `osc52(error)`.

### PS-12: The List-mode status row and form status lines are clipped to one row. LOW-MEDIUM
- **Render.**
  - `body_render.rs:110-129` `paint_list_status` paints `view.status` on exactly one row (:124).
  - Form status: `profile_form_render.rs:144-147`, one `Line`.
  - EditApiKey status: `detail.rs` (one `Line`).
- **Sources.**
  - `"✗ list failed: {e}"` (`dispatch_provider_settings.rs:57`).
  - `"✗ {e}"` from save, refresh and delete (`:98, :123, :170, :212`).
  - The profile save errors.
  - The logout failure (`app/oauth_disconnect.rs:13-18`).
- **What the user sees.** Backend errors are clipped at the terminal width and can't be copied. Ctrl+C in List mode is consumed; `tests/provider_settings_input_copy_prov138.rs:284` even asserts that it copies nothing.
- **Fix.** Wrap the status to at most 3 rows (`wrap_to_width`) and reserve that many rows from the list area. When `status` is non-empty, let Ctrl+C in List mode copy the status. That changes what the PROV-138 test asserts at :284, so update it.
- **Test.** Dispatch `ProviderSettingsStatus("✗ " + 200 chars)`, render at 80x24, and assert the tail is visible. Press Ctrl+C and assert `osc52(status)`.

### PS-13: Status is not painted in any OAuth mode, so a copy or open confirmation has nowhere to go. LOW (needed for PS-1 and PS-2)
- **Where.** `body_render.rs:53-102`. None of the OAuth renderers take `view.status`, so a `ProviderSettingsStatus` that arrives during an OAuth mode is invisible.
- **Fix.** Pass `&view.status` into `render_headless_code_entry`, `render_device_waiting` and `render_error`, and paint it as a dim line under the hint. Set it from the PS-1, PS-2 and PS-8 handlers to "✓ URL copied" or "Could not open browser: …".
  - OSC 52 gives no acknowledgement, so the confirmation should say "Copied (if your terminal supports OSC 52)" or similar.
  - See CP-14 for the app-wide version.
- **Test.** After PS-1's `c`, `render_screen` contains "copied".

### PS-14: "Refresh models" (`r`) and the Detail/OAuthNotice screens are unreachable. LOW (look-alike, group 5)
- **Where.**
  - `DetailSub::Summary` is created only by the legacy fallback at `list.rs:110-116`, which runs only when `nav_items` is empty. `apply_provider_credentials` (`settings_config.rs:6-21`) always builds `nav_items` in production.
  - `enter_on_nav_item` (`list_actions.rs:53-137`) never enters Summary.
  - So `r` → `RefreshProviderModels` (`detail.rs:67-76`) and its real handler (`dispatch_provider_settings.rs:151-175`) can't be reached, and the footer "r: refresh models · Esc: back" (`footer_hints.rs`) is never shown.
  - The same applies to `DetailSub::OAuthNotice`. Its text, "{provider_id} uses OAuth which is not yet supported in the Rust frontend" (`detail.rs:268`), is stale since PROV-113/114.
  - The `list_actions.rs:18-27` module doc is also stale: it says the login rows route to `OAuthNotice`.
- **Inline test result.** `ProviderTestResult` / `set_test_result` (`test_result.rs:57`) has no production caller, and `TestProviderConnection` has no emitter (deliberately, RPC-154). So the row-level test decoration is dead too.
- **Fix.** Either add `r` on Provider rows in List mode and advertise it, or delete Summary, OAuthNotice, the legacy fallback and `test_result`. Fix the module doc either way.
- **Test.** A source-shape test that `DetailSub::OAuthNotice` is not constructed outside tests. Or, if `r` is added, press `r` on a Provider row, `drain`, and assert `refresh_models_cache` was called once on `MockBackend`.

### PS-15: Smaller List-mode footer mismatches. LOW
- `footer_hints.rs` `footer_hint_for`:
  - The ApiKey row says "d: delete", but `open_delete_confirm` (`list_actions.rs:220-232`) does nothing silently when the provider isn't configured.
  - The Provider row says only "Enter: expand", but `d` on a Provider row also opens delete-credentials (`list_actions.rs:171`), and ←/→ expand and collapse (`list.rs`). Neither is advertised.
- **Fix.** Hide "d: delete" when the provider is not configured, and advertise `d` and ←/→ on Provider rows.
- **Test.** Render a List with an unconfigured provider whose ApiKey row is focused, and assert the footer has no "d: delete".

Also not a bug, but note it: typed code characters are not filtered (`oauth_login.rs:169-171` accepts any `Char`), while the `oauth_copilot.rs:139-141` comment says the code-entry filter is "mirrored". When adding paste (PS-4), apply the same printable-ASCII filter to both.

---

## 5. Group 1: dead actions and broken emit paths (outside Provider Settings)

Full detail is in Appendix A. The headlines:
- **DA-5 (HIGH):** the `/merge-worktree` Merge and Discard buttons do nothing.
  - Cause: `app/dispatch_merge_worktree.rs:107` `MergeConfirmDialog::new(..)` has no `.with_action_tx(self.action_tx.clone())`, so `emit_outcome` (`views/agent/merge_confirm_dialog_dispatch.rs:48-53`) sends nothing and only closes the dialog.
  - Fix: one line, following the `dispatch_worktrees.rs:71` and `dispatch_role_dialog.rs:40` pattern.
  - Test: `mock.merge_session_worktree_calls() == 1` after Enter on Merge.
- **DA-6 (LOW):** `/debug` with no session sends `Action::Custom` (`app/dispatch_slash_debug.rs:58-63`), and nothing handles it. A slash pick with `session: None` only does `tracing::warn!` (`dispatch_slash_commands.rs:41-46`).
- **DA-7:** the same as ST-1 (WebSocket OAuth stubs).
- **DA-8 (LOW):** `w` and Settings→Workspaces do nothing when the workspace list is empty (`views/board/workspace_dropdown.rs:132-133`, `app/dispatch_workspaces.rs:17-21`). There is no way to add a first workspace.
- **DA-9 (CLEANUP):**
  - `ReEnableMouseTracking` is an empty arm (`app/dispatch.rs:155-157`), and its only emitter is never constructed.
  - `ManualReconnect` can only come from the DisconnectDialog, which is never shown since RPC-416.
  - `Redraw`, `LoadWorkUnits` and `FocusNext` have no emitter and no handler.
  - 13 more variants have a handler but no emitter.
  - `RequestDeleteSession` is a deliberate no-op (`app/dispatch_resume_search_views.rs:150-156`).
  - `try_dispatch_slash_debug` always returns false.
- Checked and not dead: the custom-model CRUD wire surface (`AddCustomModel`, `EditCustomModel`, `DeleteCustomModel`) is real end to end. No view-event translator drops an `Emit`.
- Not verified: in Mux mode with no Board pane, the Settings→Daemons and Workspaces dropdowns may be invisible and stuck.

## 6. Group 2: advertised keys that don't work

Full detail is in Appendix B. The Provider Settings ones are PS-5, PS-8 (missing keys) and PS-15 above.
- **AK-1 (HIGH):** unhandled keys pass through modal dialogs to the view behind them. Cause: `compositor.rs:150-160` returns ignored, then Stage 3 `navigator.handle_event` runs (`app/events.rs:117`), and the Critical guard at `:137` protects only Stage 4.
  - Affected: ErrorDialog (`components/error_dialog.rs:79`), Notification, Status (Esc while Restoring), Help (`help_dialog.rs:168,183`), "Close Agent?", "Exit fspec?", Create Session and Attachment Picker.
  - Fix: return `consumed` before Stage 3 when the topmost layer is modal or critical. Keep Ctrl+D working.
- **AK-2 (HIGH, stub):** blocklist "Enter/Space: Toggle Rule" (`views/blocklist/render.rs:24-25`) only flips a set that rendering reads (`app/dispatch_blocklist.rs:73-79`). There is no backend RPC, so the rule stays enforced.
- **AK-3 (MED):** "(Enter to view full)" (`store/agent_view/diff_codec.rs:169-171`). Enter does that only in turn-select mode (Tab first).
- **AK-4 (MED):** agent help rows that don't match:
  - "End Bottom" is actually page down (`views/agent/dispatch.rs:211`).
  - "Home Top" works only when the input is empty.
  - "Alt+Enter Newline" queues the draft while busy.
  - "Shift+←/→ sessions" moves pane focus in mux.
- **AK-5 (MED):** agent "ESC Back" (`help_content.rs:79`) interrupts, clears the draft, or opens "Close Agent?". It never goes back.
- **AK-6 (LOW-MED):** the board footer "ESC Back" is actually Exit-confirm. The compact footer's "↵ Work Agent" actually opens the chooser. `b` (New Bot) isn't listed.
- **AK-7 (MED):** the key column in menu dropdowns and menu-bar help (`. / D A b F C M P s w`) is dead while the menu is open. "? Help" opens board help, not menu-bar help.
- **AK-8 (LOW-MED):** `a`, `w`, `[` and `]` on the board fail silently (no attachments, workspaces loading, reorder errors only logged).
- **AK-9 (LOW):** the model selector footer doesn't change in filter mode, and omits Tab and a/e/d.
- **AK-10 (LOW):** the HITL empty-submit hint "press Esc to go back" is wrong in freeform-only mode, where Esc cancels.

## 7. Group 3: copy gaps

Full detail is in Appendix C, Card A. The Provider Settings ones are PS-1, PS-3, PS-8, PS-11 and PS-12.
- **CP-1:** the Session error dialog (the main error surface for about 45 `SessionOperationFailed` emitters) can't be copied (`components/session_storage_dialog.rs:69-95`).
- **CP-2:** the API Error dialog can't be copied (`components/error_dialog.rs:63-80`).
- **CP-3:** the Notification dialog can't be copied.
- **CP-4:** the board card details dialog (description, acceptance criteria, ids, paths) has no copy (`components/board_card/events.rs:51-80`).
- **CP-5:** the board strip copies only the truncated on-screen rows.
- **CP-6:** scrollback and turn-modal copies put `\n` at every soft-wrap point (`views/agent/scrollback_copy.rs:30-44`, `turn_modal_select.rs:197-213`), so long URLs and commands paste broken. A selection can't be longer than the viewport, and there is no "copy whole turn" key, even though `full_text_for_seq` exists (`scrollback_select.rs:117`).
- **CP-7:** changed files and checkpoints can't copy a path or diff.
- **CP-8:** session worktree paths can't be copied.
- **CP-9:** the model selector can't copy a model id, and paste into its custom-model form is dropped (`navigator_events.rs` `handle_model_selector_event` ignores `Event::Paste`).
- **CP-10:** blocklist rule details can't be copied.
- **CP-11:** the viewer URL is missing from the "browser launch failed" error (`app/board_card_attachments.rs:222-226`).
- **CP-12:** the merge-success commit sha disappears at once.
- **CP-13 (needs checking):** HITL, pause and stdin prompt headers can't be selected.
- **CP-14:** there is no copy confirmation, and copies go through split writers (the board's own `Osc52Clipboard`, and scrollback writing directly). On tmux without `set-clipboard on`, every copy looks dead, even after the PS-1 fix.
- **CP-15 (from the AK fork):** agent-view Ctrl+C always emits `Interrupt` (`views/agent/dispatch.rs:198-201`), even with an active composer or scrollback selection, so there is no keyboard copy in the agent view. Fix direction: when `text_selection_active` or the composer has a selection, emit `CopyToClipboard(selection)` instead of interrupting.

## 8. Group 4: wrap and truncation gaps

Full detail is in Appendix C, Card B. The Provider Settings ones are PS-6, PS-7, PS-11 and PS-12.

Root cause for dialogs: `components/dialog_theme.rs:95-140` makes a dialog `min(content+4, area.width)` wide, and `dialog_theme_rows.rs:157` `paint_left_aligned` paints each row on exactly one line. So a row shows **76 characters at 80 columns and 116 at 120**, and a `\n` in a message paints a raw newline cell.

Root fix: add a `wrapped_rows(text, style, max_width)` helper built on `wrap_to_width`.

- **WR-1:** ErrorDialog: 65 characters of the error are visible at 80 columns and 105 at 120, and `\n` corrupts the display.
- **WR-2:** Notification dialog.
- **WR-3:** the checkpoint restore confirmation hides "overwrites the working copy" when the path is longer than about 28 characters at 80 columns (about 68 at 120).
- **WR-4:** session worktree rows (about 126 characters or more) are clipped at 80 and 120 columns, and the dialog doesn't scroll.
- **WR-5:** the blocklist detail pane (about 39 columns at 80, about 59 at 120) doesn't wrap.
- **WR-6:** `diff_common/row.rs:33-47` `truncate_path` keeps the head and drops the file name, and diff lines are clipped.
- **WR-7:** the exec-stdin notice hides "Ctrl+O answer · Ctrl+X dismiss" when the command is longer than about 20 characters at 80 columns.
- **WR-8:** the model selector write error is on one row.
- **WR-9:** the board card footer is 100 columns, so it is cut below 102.
- **WR-10:** the add-workspace and role dialog drafts and errors are on one row.
- **WR-11:** full-screen footers are on one row (the blocklist footer is exactly 80 columns).
- **WR-12 (from the AK fork):** the board footer (114 cells compact, 133 wide, `views/board/footer.rs:30,33`) is cut off at 80 and 100 columns.

## 9. Group 5: UI wired to a stub

Full detail is in Appendix C, Card C.
- **ST-1 (= DA-7):** `WebSocketFspecBackend` implements none of the `oauth_*` calls. The trait defaults in `transport/backend/credentials.rs:57-130` return `Unimplemented` or "… OAuth login is not supported".
  - The RPC service does declare them (`rust/rpc/src/service_declaration/supervision_credentials.rs:140-180`).
  - Only the Browser rows are gated (`supports_browser_oauth`, `app/dispatch_provider_settings.rs:32`). In client mode (`rust/fspec/src/client.rs:59`) and after a daemon switch (`daemon_connect.rs:43`), "Sign in with code", the device rows, Copilot and Logout always fail. Logout shows a misleading storage error (`app/oauth_disconnect.rs:13-18`).
  - Fix: forward the RPCs, using the `websocket/session_context.rs:88-120` shape, or add a `supports_oauth_login()` capability that gates the rows.
- **AK-2:** the blocklist toggle is UI-only (see group 2).
- **ST-2:** the mouse-capture release toggle (`mouse/toggle.rs`) is scaffolding, and the `ReEnableMouseTracking` arm is empty.
- **ST-3 (low, documented):** the WebSocket transport has no checkpoint-progress or git-state push.
- **ST-4 (info):** `StatusDialog` is used only in tests. The `rust/tui` `StatusDisplay` and event stream are unused.
- **PS-14:** "refresh models" and the stale "OAuth not yet supported in the Rust frontend" screen are unreachable.

---

## Appendices

Appendix A, B and C are the three subagent reports, kept word for word except for the line-number corrections described below. Where they overlap with section 4:
- DA-1 to DA-4 in Appendix A are the same findings as PS-1 to PS-4. Section 4 is the authoritative version.
- **Line-number corrections in Appendix A**, already applied below. The subagent cited offsets from concatenated output. The correct references are:
  - `oauth_login_render.rs`: :101 (Visit line), :105 (hint), :81-82 (device code and URL), :133 (error line).
  - `paste.rs:17-36`, with the ignore arm at :35.
  - `copy.rs:43` for `is_copyable_mode`.

---

## Appendix A: dead actions (full)


Scope: all 281 `Action` variants composed from `rust/fspec-tui/src/components/action_*.rs` (via `components/mod.rs:115-151`). Each variant's emitters and handlers were traced, mechanically and then by hand, through `app/dispatch.rs`, the `try_dispatch_*` chain (`app/dispatch_capability.rs:17-42`), `Navigator::apply_action` (`views/navigator.rs:172`, `views/navigator_auxiliary.rs`), `Compositor::update`, and the view-event translators (`views/navigator_events.rs`, `views/navigator_mux_routing.rs`, `views/agent/dispatch_mode_views.rs`, `views/agent/mouse_dispatch.rs`). `rust/tui` and `cli/src/interactive` have no Action bus and no key handling (`rust/tui/src/status.rs`'s "ESC to interrupt" string is never used), so they have nothing to report for this group.

Test-harness names used below:
- `tests/common::MockBackend` and its counters: `merge_session_worktree_calls()` / `discard_session_worktree_calls()` (`tests/common/mock_scripting_6.rs:13,25`) and `seed_oauth_headless_start()` (`mock_scripting_5.rs:12`).
- `App::new(backend)`, `App::handle_event(&Event)` / `App::handle_paste(&str)` (`src/app/composer_draft.rs:97,105`), `App::try_recv_action()` and `App::next_pending_task()` (`src/app/state/state_accessors.rs:123,182`).
- `App::set_clipboard_writer_for_test(Box<dyn Write+Send>)` (`src/app/dispatch_scroll.rs:248`).
- `App::set_board_attachment_launcher_for_test(launcher)` (`src/app/board_card_attachments.rs:159`).
- The `SharedWriter` / `app_in_provider_view` / `drain` / `osc52` helpers in `tests/provider_settings_input_copy_prov138.rs:109-160`.
- `fresh_app` / `drain_pending` / `wait_until` in `tests/provider_settings_oauth_login_prov113.rs:556-586`.

---

## DA-1: `c: copy URL` on Anthropic "Sign in with code" does nothing (OAuthCopyUrl only logs). Severity HIGH (product owner's report)
- Emitter: `views/provider_settings/oauth_login.rs:159-163`. `KeyCode::Char('c')` with `code_input` empty emits `Action::OAuthCopyUrl { url }`.
- Routing: `app/dispatch_provider_settings.rs:284-286` sends it to `try_dispatch_oauth`.
- Handler: `app/dispatch_provider_settings_oauth.rs:78-80`. The arm is only `tracing::info!(url, "oauth copy-url requested")`.
- Hints: the body line `oauth_login_render.rs:105` says "Enter: submit · c: copy URL · o: open URL · Esc: cancel". The footer `footer_hints.rs:39-40` says "Enter: submit · c: copy · o: open · Esc: cancel".
- What the user sees: pressing `c` gives no clipboard write and no feedback. The URL on screen is also clipped (`render_headless_code_entry` uses `Paragraph` without `.wrap`, `oauth_login_render.rs:101`; the wrap group covers this).
- Why the tests missed it: `tests/provider_settings_oauth_login_prov113.rs:487` (`headless_code_entry_c_copies_only_while_empty`) only checks that the view emits `OAuthCopyUrl`. It never checks the clipboard.
- Fix: in the arm, call `self.handle_copy_to_clipboard(url.clone())` (`app/dispatch_scroll.rs:234`, which writes through `Osc52Clipboard`), then set the view status to something like "URL copied" (`navigator.provider_settings.set_status`). An alternative is to delete `OAuthCopyUrl` and have the view emit `Action::CopyToClipboard(url)` directly, as `copy.rs:81` already does.
- Test (App dispatch): `fresh_app(mock)` with `mock.seed_oauth_headless_start(URL, "v")`, then `set_clipboard_writer_for_test(SharedWriter(buf))`. Dispatch `OpenProviderSettingsView`, then dispatch `Action::OAuthHeadlessReady { provider_id: "anthropic", authorize_url: URL, pkce_verifier: "v", generation: app.navigator().provider_settings.oauth_generation() }`. Then `handle_event(Key 'c')` and `drain`. Assert that `buf == osc52(URL)` and contains the full URL, not a truncated one.

## DA-2: `o: open URL` does nothing (OAuthOpenUrl only logs). Severity HIGH
- Emitter: `views/provider_settings/oauth_login.rs:164-168`.
- Routing: `dispatch_provider_settings.rs:284`.
- Handler: `dispatch_provider_settings_oauth.rs:75-77`, which is `tracing::info!` only.
- Hints: the same two strings as DA-1 (`oauth_login_render.rs:105`, `footer_hints.rs:39-40`).
- What the user sees: `o` opens no browser and shows no feedback.
- Fix: reuse the existing browser-launcher seam: the `AttachmentLauncher` type (`app/board_card_attachments.rs:26`), its default `open::that` (`:80`), and its `spawn_blocking(move || launcher(&url))` (`:223`). Lift the launcher to an App-level `url_launcher`, or call `self.board_card.launcher` directly. On `Err`, set the view status to "Could not open browser: … (press c to copy)".
- Test: `set_board_attachment_launcher_for_test(Arc::new(move |u| { rec.lock().push(u.to_string()); Ok(()) }))`, put the view in code-entry mode as in DA-1, then `handle_event(Key 'o')`, `drain`, and `next_pending_task().await`. Assert that `rec == [URL]`.

## DA-3: Ctrl+C is swallowed in every OAuth mode before any copy can be emitted. Severity MEDIUM (overlaps the copy group)
- Consumer: `views/provider_settings/mod.rs:193-201`. `copy::intercept_ctrl_c` runs first, but `is_copyable_mode` (`copy.rs:43-55`) excludes `OAuthHeadlessCodeEntry`, `OAuthDeviceWaiting`, `OAuthError`, `OAuthEnterpriseUrlEntry` and the rest. The blanket `CONTROL|ALT => Consumed` at `mod.rs:196-201` then eats the key.
- What the user sees: Ctrl+C silently does nothing on the authorize URL, the device `user_code` / `verification_url` (`oauth_login_render.rs:81-82`), and the error text (`oauth_login_render.rs:133`).
- Fix: add these modes to `is_copyable_mode` and have `handle_copy` return the most useful text for each mode: the authorize URL (or `code_input` once it is non-empty), `user_code` for device mode, the `error` string, and `url_input`. Do not mask any of these.
- Test: the same harness as DA-1, sending Ctrl+C in each mode. Assert the OSC 52 payload.

## DA-4: Paste is dropped in the OAuth code-entry and enterprise-host entry modes. Severity HIGH (the code has to be pasted from the browser)
- Consumer: `views/provider_settings/paste.rs:17-36`. Only the profile forms and `EditApiKey` take paste; everything else falls to `_ => ProviderSettingsEvent::Ignored`.
- Translator: `views/navigator_events.rs:38-41` maps that to `EventResult::ignored()`. `App::route_paste` (`app/events.rs:219-247`) then drops it.
- What the user sees: after copying the `code#state` string from claude.ai, pasting into "Code:" does nothing, so the user has to type a code of roughly 100 characters by hand. GitHub Enterprise "URL or domain:" also rejects paste.
- Fix: add paste arms for `OAuthHeadlessCodeEntry { code_input }` (append the pasted text trimmed of whitespace and newlines, then rebuild the mode the way `handle_code_entry_key` does at `oauth_login.rs:181`) and for `OAuthEnterpriseUrlEntry { url_input }` (route through the copilot preamble's char filter).
- Test: put the view in code-entry mode as in DA-1, then `app.handle_paste("abc#xyz\n")`. Assert that the mode's `code_input == "abc#xyz"`. Then `handle_event(Enter)` followed by `try_recv_action()` should give `OAuthLoginHeadlessSubmit { code: "abc#xyz", .. }`.

## DA-5: The `/merge-worktree` confirm dialog's Merge and Discard buttons do nothing. Severity HIGH
- Emitter: `views/agent/merge_confirm_dialog_dispatch.rs:48-53`. `emit_outcome` sends `MergeConfirmed` / `DiscardConfirmed` / `CancelMergeDialog` only `if let Some(tx) = self.action_tx`, and its self-remove callback closes the dialog either way.
- Root cause: `app/dispatch_merge_worktree.rs:107` builds `MergeConfirmDialog::new(session_id, summary)` without `.with_action_tx(..)` (that method exists at `views/agent/merge_confirm_dialog.rs:93`). `take_pending_outcome` (`merge_confirm_dialog.rs:129`) is only read in tests.
- Handlers, which are real but never reached: `dispatch_merge_worktree.rs:185-193` (`handle_merge_confirmed` at :116, `handle_discard_confirmed` at :145).
- Render: "Merge Worktree" with buttons [Merge] [Discard] [Cancel] (`merge_confirm_dialog.rs:124,208,243`).
- What the user sees: Enter or a click on Merge (or Discard) closes the dialog, and then nothing happens. The worktree is not merged or discarded, and no notice appears. Cancel looks fine only because the dialog removes itself.
- Why the tests missed it: `tests/merge_worktree_rpc057.rs:235+` and `tests/tui112_dialog_click_activation.rs:331,360` build the dialog directly and read `take_pending_outcome`. The App path is never exercised.
- Fix: add `.with_action_tx(self.action_tx.clone())` at `dispatch_merge_worktree.rs:107`, the same pattern as `SessionWorktreesDialog` (`app/dispatch_worktrees.rs:71`) and `RoleDialog` (`app/dispatch_role_dialog.rs:40`).
- Test: `App::new(MockBackend)` inside a tokio runtime. Dispatch `Action::OpenMergeConfirmDialog { session_id, summary }` with a non-zero summary, then `handle_event(Key Enter)`. Assert that `try_recv_action()` is `MergeConfirmed`, dispatch it, and await `next_pending_task()`. Assert `mock.merge_session_worktree_calls() == 1`. Repeat with Right then Enter (Discard) and assert `discard_session_worktree_calls() == 1`.

## DA-6: `/debug` with no session sends `Action::Custom`, which nothing handles, so the notice is lost. Severity LOW
- Emitter: `app/dispatch_slash_debug.rs:58-63`, which sends `Action::Custom("[notice] [debug] capture toggled → …")`.
- Handler: none. There is no arm in `dispatch.rs`, no claim in the `try_dispatch_*` chain, no `apply_action` arm, and no component `update` matches `Custom`. The variant's own doc (`components/action_runtime.rs:9-11`) says the App fans it out "without interpreting".
- What the user sees: the debug flag toggles silently.
- Related log-only case: `app/dispatch_slash_commands.rs:41-46`. A slash pick with `session: None` after focus has moved only does `tracing::warn!`.
- Fix: push `NotificationDialog::info(text).with_action_tx(self.action_tx.clone())` (`components/notification_dialog.rs:145`), or reuse `Action::SessionOperationFailed`'s surface. Then remove `Action::Custom`.
- Test: `App::new(MockBackend)` with no session. Dispatch `Action::SlashCommandSelected(None, SlashCommandAction::Debug)` and `drain`. Assert that the compositor contains the notification id, and that rendering to a `TestBackend` at 80x24 shows "capture toggled".

## DA-7: OAuth rows lead to stub backend calls on the WebSocket transport (client mode and remote daemons). Severity MEDIUM (overlaps the stubs group)
- Every OAuth method falls through to the `FspecBackend` trait default. There are no overrides in `transport/websocket*`; only `transport/embedded` implements them.
  - `transport/backend/credentials.rs:57-122`: `oauth_clear_tokens` and `oauth_get_tokens` return `Unimplemented`.
  - `oauth_headless_start`, `oauth_headless_complete`, `oauth_device_start`, `oauth_device_poll` and `oauth_copilot_device_start` return "… OAuth login is not supported".
- The view still shows "Sign in with code" and the device rows (`views/provider_settings/projection.rs:47-58`). Only the Browser rows are gated, by `supports_browser_oauth` (`nav_tree_ops.rs:125`, `dispatch_provider_settings.rs:32`).
- Where this transport is used: `fspec/src/client.rs:59` and the daemon switcher (`fspec-tui/src/daemon_connect.rs:43`).
- What the user sees: Enter on "Sign in with code" shows "OAuth Login error / headless OAuth login is not supported", with "Enter: retry", which can never succeed. Logout fails the same way.
- Fix: implement the WebSocket RPCs for these calls, or add a `supports_oauth_login()` capability (default `false`, `true` in embedded) and gate the Headless and device rows on it, the same way the Browser rows are gated.
- Test: a `MockBackend` variant that returns `false` from the new capability. After `OpenProviderSettingsView` plus `ProviderCredentialsLoaded([anthropic oauth info])`, assert that no `NavItemKind::OAuthLogin { method: Headless }` row exists.

## DA-8: `w` / Settings→Workspaces silently does nothing when the workspace list is empty. Severity LOW
- Emitter guard: `views/board/workspace_dropdown.rs:132-133` (`&& !store.workspaces().is_empty()`).
- Handler guard: `app/dispatch_workspaces.rs:17-21`.
- Menu path: `components/menu_bar/items.rs:107`.
- Hint: `components/help_content.rs:44` says "w Workspaces: switch, add, remove".
- What the user sees: when the listing is empty or failed, `w` and the menu entry do nothing. The "a: add" action lives only inside the dropdown, so the user has no way to add the first workspace.
- Fix: open the dropdown even when empty (show "No workspaces · a: add · Esc: close"), or emit a notice.
- Test: `App::new(MockBackend)`, dispatch `WorkspacesLoaded(vec![])` then `OpenWorkspaceDropdown`. Assert `board_store().workspace_dropdown().is_some()`, or assert the notice text in a render at 100x30.

## DA-9: Unreachable or vestigial variants. Severity CLEANUP (no visible effect today)
- `ReEnableMouseTracking`: the arm is empty (`app/dispatch.rs:155-157`). The only emitter is `mouse/toggle.rs:97`, inside `MouseTrackingToggle`, which is never constructed outside tests.
- `ManualReconnect`: the handler is real (`dispatch.rs:43`), but the only emitter is the DisconnectDialog stage-1 `r` key (`app/events.rs:164-165`). `DisconnectDialog` is never pushed in production since RPC-416 (`components/disconnect_dialog.rs` is constructed only in tests).
- `Redraw`, `LoadWorkUnits` and `FocusNext` (`action_runtime.rs:8,15,69`) have no emitter and no handler.
- These have handlers but no emitter:
  - Intentional: `TestProviderConnection` (dropped on purpose by RPC-154, `views/provider_settings/detail.rs:56-66`), and `DeleteProviderCredentials` / `DeleteProfile`, which are superseded by the `ConfirmDelete*` variants.
  - Not intentional as far as the code shows: `SendToSubordinate`, `OpenResumeView`, `NavigationTargetSet`, `AttachSession`, `FocusPrevColumn`, `FocusNextColumn`, `PauseResumed`, `HitlSubmitted`, `ExecStdinPromptFetched`, `OpenThinkingDialog`, `SessionResumeComplete`, `AttachWorkUnitToSession`, `WorkUnitAttached`. None of these is advertised in any hint.
- `try_dispatch_slash_debug` (`app/dispatch_slash_debug.rs:93`) always returns `false` and is `#[allow(dead_code)]`.
- Fix: delete these, or wire them up. A source-shape test can check that every Action variant has at least one non-test emitter.

### Checked and not dead
These look empty but are handled elsewhere, so they are not findings:
- `CloseChangedFilesView` (`dispatch_changed_files.rs:178`) and `CloseModelSelectorView` (`dispatch_model_selector.rs:198`): the Navigator flips the view (`navigator_auxiliary.rs:36-48`).
- `BoardCardDetailsLoaded{Err}` and `BoardCardAttachmentFailed` (`dispatch_board_card.rs:101`): the dialog handles them in `components/board_card/events.rs:117-122`.
- Mux `MenuZoneCActivate` (`dispatch_menu_mux.rs:111`): the mux bar paints no Zone C.
- `OpenAgentView(None)` in the Navigator: the App handles it at `dispatch_session_cycle.rs:53`.
- `handle_request_delete_session` and `handle_close_blocklist_view`: the view already owns the state.
- The custom-model CRUD "wire surface" (`AddCustomModel`, `EditCustomModel`, `DeleteCustomModel`) is real: emitted at `views/model_selector/crud.rs:105-125,145-158`, handled at `app/dispatch_model_selector.rs:100-162`, and backed in both transports (`transport/websocket/history_models.rs:88-131`, `transport/embedded/models_profiles.rs:35-72`).
- View-event translators: no `Emit` is dropped. Mouse handlers return only Consumed or Ignored, and the `_ =>` arms in `navigator_events.rs` and `mouse_dispatch.rs` never see `Selected` or `Emit`.

Unverified, needs a check: in Mux mode, Settings→Daemons and Settings→Workspaces open board-store dropdowns that only the Board pane paints and only the Board pane's keys can close. With no Board pane in the mux layout, they may be invisible and stuck.

---

## Appendix B: advertised keys (full)


Scope: every footer, hint, help row and inline key hint in rust/fspec-tui/src outside views/provider_settings, traced through
App::route_event (app/events.rs): Stage 1 disconnect, Stage 2 Compositor, Stage 3 Navigator (with the mux Shift+←/→ intercept),
and Stage 4 app shortcuts (`?`, board Esc, Ctrl+D). Stage 4 is skipped only when the topmost layer is Critical.
All paths are relative to rust/fspec-tui/src unless they say otherwise.

About 55 hint surfaces were checked. These were verified working and are not listed below: changed-files footer; checkpoints footer and its
restore/delete y/n and "DELETE ALL" dialogs; slash and file popups; resume and search views; turn modal; merge-confirm and
session-worktrees dialogs; daemon switcher (`s`, a/e/d, Enter, Esc); daemon form and remove dialogs; workspace dropdown a/d;
mux config dialog (A, ⌫, S, Enter, Esc); thinking-level dialog (D, Enter); role dialog (Ctrl+D clear); create-session,
attachment-picker, board-card chooser and board-card frame; session-storage dialog; missing-profile-limit dialog; add-workspace dialog; stdin notice and answer
box (Ctrl+O, Ctrl+X, Enter, Esc, Ctrl+C, Ctrl+D); pause prompt (←/→, Enter, Esc=Deny, Y/N); HITL options footer; busy-composer
hints; model-selector form and delete-confirm footers; and board help rows for f/c/d/m/p/s/u/./[/]/Shift+Right/PgUp/Home/End.

Test harness: tests/common/mod.rs has `MockBackend`, `test_app(backend)` (an 80x24 TestBackend), `render_one_frame` and
`buffer_to_rows`. tests/common/harness.rs has `AppTestHarness::new()`, which seeds session s-1 and opens the Agent view.
It also provides `press_key`, `key`, `key_mod`, `active_view`, `compositor_contains`, `should_quit`, `input_value` and `set_input`.
On `App` you also have `handle_event(&Event)`, `handle_paste`, `dispatch`, `try_recv_action`, `compositor()`,
`agent_view_store()`, `board_store()`, `navigator()` and `set_clipboard_writer_for_test`. Scrollback position is exposed through
`ScrollbackList::scroll_state()`, which carries `offset` and `stick_to_bottom`.

---

## AK-1: Modal dialogs pass unhandled keys to the view hidden behind them (HIGH)
- Root cause: compositor.rs:150-160 returns `ignored` when no layer consumes a key. app/events.rs:117 then always runs
  Stage 3 `navigator.handle_event`. The Critical guard at events.rs:137 protects only Stage 4.
- Dialogs whose `handle_event` falls through to `EventResult::ignored()` for unhandled keys:
  - components/error_dialog.rs:79, footer "Press ESC to dismiss" (error_dialog.rs:93), Critical. It is pushed for every
    stream `API Error` by app/dispatch_dialog_dismiss.rs:55-60.
  - components/notification_dialog.rs:256, footer "Press ESC to dismiss" / "Closing in Ns... (ESC to dismiss)".
  - components/status_dialog.rs:229. While Restoring, Esc is deliberately `ignored` (status_dialog.rs:215), so the Esc falls
    through to the view behind it.
  - components/help_dialog.rs:183 (`_ => {}` at :168), footer "ESC to close", Critical.
  - components/exit_confirmation_dialog.rs:251 ("Close Agent?", Critical), footer "← → Navigate | Enter Select | Esc Cancel".
  - components/board_exit_confirmation_dialog.rs:233 ("Exit fspec?").
  - components/create_session_dialog.rs:271, components/attachment_picker_dialog.rs:178.
- What the user sees:
  - An API Error dialog covers the agent view. The user types "ok" and presses Enter. The text goes into the hidden composer
    and is sent as a message (`Action::InputSubmitted`).
  - With Help open on the board, pressing `p`, `f` or `c` opens Provider Settings, Changed Files or Checkpoints under the dialog.
  - With "Close Agent?" open, pressing Tab toggles turn-select mode underneath, and Shift+→ switches sessions underneath.
  - The dialogs say "Press ESC to dismiss" and look modal, but other keys reach the view behind them.
- Fix direction: make these Critical and Foreground modals consume every key event, the way menu_bar_help.rs:225 and
  daemon_remove_confirm_dialog.rs:66 already do. A simpler, more central fix: in `route_event`, when
  `topmost_is_critical` (or the topmost layer reports modal) and the compositor did not consume the key, return
  `consumed` before Stage 3. Keep Ctrl+D quit by special-casing it.
- Test: `AppTestHarness::new()`; `app.dispatch(Action::ChunkReceived(s1, StreamChunk::Error{..}))` → assert
  `compositor_contains("error-dialog")`; `press_key(key(Char('o')))`, `press_key(key(Enter))`; drain `try_recv_action()`
  → assert NO `InputSubmitted` and `input_value()==""`. Board variant: `AppTestHarness::empty()`, press `?`, then `p` →
  assert `active_view()==ViewMode::Board`.

## AK-2: Blocklist "Enter/Space: Toggle Rule" never reaches enforcement (HIGH; also a group-5 stub)
- Hint: views/blocklist/render.rs:24-25, "Enter/Space: Toggle Rule".
- Key handling: views/blocklist/mod.rs:218,220 → `toggle_focused` (:280) → `Action::ToggleBlocklistRule`.
- Handler: app/dispatch_blocklist.rs:73-79 only flips `AgentViewStore.blocklist_disabled_by_session`
  (store/agent_view/blocklist_state.rs:37). That set is read only by render code (views/navigator_render.rs:50,
  views/blocklist/panes.rs:81-118). No backend call exists; the only blocklist RPC is `blocklist_list`. The module doc admits
  "disabled rules are purely a UI affordance and do NOT flow back into the tool-execution path".
- What the user sees: the rule turns grey with "(disabled)", but the agent's tools are still blocked by it.
- Fix direction: either add a backend RPC (`blocklist_set_session_disabled(session, rule_id, bool)`) and spawn it from
  `handle_toggle_blocklist_rule`, or remove the toggle and change the hint (for example, show "(view only)").
- Test: with MockBackend recording calls, open the view (`Action::OpenBlocklistView`) and dispatch
  `BlocklistRulesLoaded(vec![rule])`. Press Space and pump `try_recv_action` into `dispatch`. Assert that the mock recorded a
  disable call for (s-1, rule.id). Today it records none.

## AK-3: Collapsed tool card says "(Enter to view full)", but Enter does nothing (MEDIUM)
- Hint: store/agent_view/diff_codec.rs:169-171 `hidden_lines_hint`. It is used by chunk_wrap.rs:241, diff_format.rs:165 and
  patch_diff.rs:77 for every collapsed tool output, Edit/Write diff and apply_patch diff.
- Key handling: in normal mode, Enter goes to the composer (views/agent/dispatch.rs:226-253). An empty input gives
  `Submitted("")` → `ignored`, so nothing happens. Non-empty input sends the message. Enter opens the full view only in
  turn-select mode (views/agent/dispatch_select.rs:58-67 → `Action::OpenTurnModal` → app/dispatch_scroll.rs:51), and only
  after the user presses Tab and walks to the right turn.
- What the user sees: pressing Enter as the hint says does nothing, or sends a half-typed draft.
- Fix direction: change the hint to "(Tab, then Enter to view full)", or make bare Enter with an empty composer open the
  last collapsed card.
- Test: seed a settled tool chunk with more than 8 lines; render (`test_app` + `render_one_frame`) and assert that the hint text matches the
  binding. Then press Enter with an empty composer and assert that no `OpenTurnModal` arrives (today's behaviour). After a
  fix, assert that `navigator().agent.turn_modal_seq.is_some()`.

## AK-4: Agent help rows that don't match the bindings (MEDIUM)
- Help: components/help_content.rs:59-91 (`agent_help_lines`, opened by `/help`, app/dispatch_slash_commands.rs:59).
- "End  Bottom" (:71): views/agent/dispatch.rs:211 maps End to `ScrollbackPageDown`, which moves one viewport
  (`scroll_focused`, app/dispatch_scroll.rs:10). It is not `jump_to_bottom` (views/agent/scrollback.rs:219). End is also always
  taken from the composer, so End-of-line never works while typing.
- "Home  Top" (:70): works only when the input is empty (dispatch.rs:217). Otherwise Home moves the cursor.
- "Alt+Enter  Newline (legacy terminals)" (:67): while Running or Compacting, Alt+Enter queues the draft instead
  (views/agent/queue_key.rs:13-25). The busy hint says this correctly, but the help does not.
- "Shift+←/→  Cycle sessions" (:73), and the composer placeholder `INPUT_PLACEHOLDER_HINT` (views/agent.rs:112-113,
  "'Shift+←/→' sessions"): in mux mode, app/events.rs:98-110 intercepts these keys to move pane focus, rotate the window, or
  prompt for a new agent at the right edge. The mux footer (views/multiplex/render.rs:251) says "focus", so two hints on the
  same screen disagree.
- Fix direction: map End to a new `Action::ScrollbackEnd` that calls `jump_to_bottom`, mirroring `ScrollbackHome` (or drop
  End when the input is non-empty). Write the sub-states into the help ("Home (empty input)", "Alt+Enter newline / queue
  while busy"). Make the placeholder mux-aware.
- Test: `AppTestHarness::new()`, seed 200 rows, call `set_scrollback_viewport_height(s1, 10)`, press PageUp ×5, then End →
  assert that `scroll_state().stick_to_bottom` is true and `offset == max`. Today it moves by only one page.

## AK-5: "ESC Back" on the Agent view never goes back (MEDIUM)
- Hint: help_content.rs:79 "ESC  Back". The Thinking/Compacting spinners also say "(Esc to stop)"
  (views/agent/input_transition.rs:70,199,202).
- Key handling: dispatch.rs:196 → `Action::AgentEscPressed` → app/dispatch_esc_cascade.rs:37-70. With a session open, Esc
  interrupts the run if it is busy, otherwise clears a non-empty draft, otherwise opens the "Close Agent?" dialog
  (`request_close`, app/dispatch_agent_close.rs:59). It reaches `BackToBoard` only when no session exists. While the stdin box is focused, Esc
  returns to the chat instead of stopping (views/agent/exec_stdin_keys.rs:62).
- What the user sees: pressing Esc to "go back" from an idle agent opens a dialog offering to close the agent.
- Fix direction: change the help to "ESC  Stop run / clear draft / close agent (confirm)". Mention that the menu bar
  "Board View" item (← on an empty input) is how to go back.
- Test: render `HelpDialog::for_agent()` and assert the new text. Press Esc on an idle s-1 with an empty draft and assert
  `compositor_contains(EXIT_CONFIRMATION_DIALOG_ID)` (needs a tokio runtime, because `request_close` spawns a task).

## AK-6: Board footer "ESC Back" and "↵ Work Agent" are wrong (LOW-MEDIUM)
- Hint: views/board/footer.rs:30 (compact) and :33 (wide).
- Esc on the board (app/events.rs:189-201) opens the "Exit fspec?" confirmation. It is Exit, not Back; the board help
  (help_content.rs:47) says "Exit (confirm)".
- In the compact footer (shown when the inner width ≤ 133), "↵ Work Agent" is wrong: Enter opens the chooser (views/board.rs:198-207 →
  `OpenBoardCardChooser`). Enter with no selected unit is `ignored`, so nothing happens.
- Neither the board help nor the footer lists the real key `b` = New Bot (views/board/keys.rs:110-113). The menu does list it
  (components/menu_bar/items.rs KANBAN).
- Fix direction: "ESC Exit", "↵ Choose (agent/details)", and add `b  New Bot` to `board_help_lines`.
- Test: `test_app` at widths 120 and 200, `render_one_frame`, `buffer_to_rows` → assert the footer row contains "ESC Exit"
  and not "ESC Back".

## AK-7: Menu key labels don't work while the menu is open (MEDIUM)
- Hint: every dropdown row paints a key column (components/menu_bar/dropdown_paint.rs:136-139, `entry.key` from
  components/menu_bar/items.rs: `. / D A b F C M P s w ? Esc`). The Menu-bar help dialog (`u`) paints "KEY Label" rows
  (components/menu_bar_help.rs:246-256).
- Key handling while the dropdown is open: views/board/menu_keys.rs:47-84 handles only ↑↓/j/k, Enter, Esc and ←→/h/l. Every
  other key returns `Some(ignored)`, so it reaches Stage 4. There only `?` does something; the other labels are dead. The mux top bar
  swallows them (views/multiplex/menu_keys.rs:158,182,206). Menu-bar help consumes them all (menu_bar_help.rs:223-225).
- The "Esc Exit" row: Esc in an open dropdown only closes the dropdown (menu_keys.rs:68-73). In menu-bar help, Esc closes
  the dialog.
- The "? Help — Show the full menu bar help" row (items.rs HELP[0]) dispatches `OpenBoardHelp`, which opens the board
  keybindings dialog, not the menu-bar help (`u`).
- In mux with an Agent pane focused, and the bar not focused, the same labels are typed into the composer.
- What the user sees: the menu shows "F  Changed Files"; pressing F with the menu open does nothing.
- Fix direction: while the dropdown or menu-bar help is open, match a plain key (case-insensitive) against
  `entry.key` and emit `MenuExecuteItem { category, row }` (or the entry's `to_action()`). Fix the HELP[0] description.
- Test: `AppTestHarness::empty()`; dispatch `MenuOpenDropdown(1)` (Tools); press `F`; pump actions → assert that
  `OpenChangedFilesView` is emitted and `active_view()==ViewMode::ChangedFiles`. Same for `u`, then `F`, in menu-bar help.

## AK-8: Advertised board keys silently do nothing in common states (LOW-MEDIUM)
- `a` / "A Attachments": views/board/keys.rs:93-103 emits only when the selected unit has attachments.
  app/dispatch_viewer.rs:62-75 (the menu path) also returns silently with no selection or no attachments.
- `w` Workspaces: views/board/workspace_dropdown.rs:132-138 does nothing while `workspaces()` is empty (still loading or a
  load error). The Settings-menu path does the same.
- `[` / `]` "Priority Up/Down" (board footer): app/dispatch_reorder.rs:28-34 and :53-55. When the workspace is not known yet
  or the backend refuses, the failure is only written to `tracing::warn!`. The user sees nothing.
- Fix direction: reuse `Action::SessionOperationFailed(String)` (as `highlighted_saved_daemon` does in
  app/dispatch_daemon_admin.rs:83-86) to show "No attachments on <ID>", "Workspaces are still loading" and
  "Could not move <ID>: …".
- Test: `AppTestHarness::empty()` with a work unit that has no attachments; press `a` → assert a notice or error dialog via
  `compositor()` or the store. For reorder, make MockBackend's `move_work_unit_up` return Err and assert the notice.

## AK-9: Model selector footer doesn't change in filter mode and omits real keys (LOW)
- Hint: views/model_selector/rows.rs:26-27 `FOOTER`, painted unconditionally at views/model_selector/render.rs:96.
- In filter mode (views/model_selector/crud.rs:168-197), Esc clears the filter instead of closing, Enter commits the filter
  instead of selecting, and `r` / `/` are typed into the filter.
- The footer never mentions Tab → Provider Settings (dispatch.rs:52), or `a` / `e` / `d` for custom models (dispatch.rs:56-67).
- "Enter Select" does nothing until an arrow key has been pressed (`has_selection`, PROV-101, dispatch.rs:153-160).
- Fix direction: when `filter_mode`, use a filter footer ("type to filter · Enter apply · Esc clear"). Append
  "Tab providers · a/e/d custom model".
- Test: open `ModelSelectorView`, press `/`, render at width 120 → assert that the footer shows the filter hint.

## AK-10: HITL "press Esc to go back" cancels the whole request in freeform mode (LOW)
- Hint: views/agent/hitl_prompt.rs:35 `EMPTY_HINT`, shown after an empty submit.
- Handler: views/agent/hitl_keys.rs:150-162. With `other == false` (a freeform-only question), Esc emits `HitlCancelled`,
  which cancels the whole request. It returns to options only in "Other" mode.
- Fix direction: use different empty hints: "press Esc to go back to options" vs "press Esc to cancel".
- Test: seed a freeform-only HITL slot and press Enter with an empty input. Render and assert that the hint names cancel.
  Press Esc and assert that `HitlCancelled` is emitted.

---

### Notes for other groups
- Group 1 (dead actions): `Action::RequestDeleteSession` is a deliberate no-op handler
  (app/dispatch_resume_search_views.rs:150-156).
- Copy group: agent-view Ctrl+C always emits `Interrupt` (views/agent/dispatch.rs:198-201), even when a composer or scrollback
  text selection is active. There is no keyboard copy anywhere in the agent view.
- Wrap group: the wide board footer (footer.rs:33, 133 cells) and the compact one (114 cells) are cut off on
  80- and 100-column terminals. "ESC Back" is the first part lost.

---

## Appendix C: copy, wrap and stubs (full)


Scope: rust/fspec-tui except `views/provider_settings` (covered by the parent), plus rust/tui and rust/cli/src/interactive.
Read-only audit. Line numbers are as of 2026-11-07. All paths are relative to `rust/fspec-tui/src/` unless stated otherwise.

---

## 0. The copy machinery that works today (reuse this)

**Writer.** `mouse/clipboard.rs` `Osc52Clipboard<W: Write + Send>`:
- `copy(text)` writes `ESC ] 52 ; c ; base64(utf8) BEL` and then flushes.
- There is no chunking and no read-back, and nothing tells the user whether the terminal accepted the copy.
- `App` owns one writer: `app/state/state_types.rs:78` (`clipboard: Osc52Clipboard<Box<dyn Write + Send>>`, stdout at :139).

**The one Action path.** Any view or dialog emits `Action::CopyToClipboard(String)`:
- The variant is declared in `components/action_transcript_selection.rs:52`.
- The arm is `app/dispatch.rs:231`, which calls `App::handle_copy_to_clipboard` (`app/dispatch_scroll.rs:234`).
- Empty text is a no-op. A copy failure is only logged with `tracing::warn`.
- Emitters today:
  - the composer selection (`views/agent/mouse_dispatch.rs:279`);
  - the turn-content-modal selection (`views/agent/turn_modal_select.rs:184`);
  - provider-settings Ctrl+C (`views/provider_settings/copy.rs:81`).

**Agent scrollback selection** uses actions rather than `CopyToClipboard`:
- `SelectionBegin`, `SelectionBeginLine`, `SelectionExtend`, `SelectionCommit` and `SelectionClear` (`action_transcript_selection.rs:30-48`).
- The handlers are in `app/dispatch_scroll.rs:190-229`. `handle_selection_commit` reads `ScrollbackList::selected_text(spans)` (`views/agent/scrollback_copy.rs:30`) and calls `self.clipboard.copy` directly.
- Gesture recognition is `mouse/gesture.rs` `SelectionRecognizer`: drag, plus a long press that selects the whole line. The selection model is `mouse/selection.rs` (`Selection::collapsed`, `whole_line`, `spans(width) -> Vec<RowSpan>`).
- The highlight is drawn by `views/agent/scrollback_paint::paint_selection_highlight`.
- Scrolling clears the selection (`app/dispatch_scroll.rs:13`, rule COPY-006 [7]).

**Board details strip** (`views/board/details_select.rs`, COPY-009):
- It uses its **own** writer, `BoardView.clipboard` (`views/board.rs:107`), with its own test seam `BoardView::set_clipboard_writer_for_test` (`views/board.rs:134`).
- It bypasses `App.clipboard`, so `App::set_clipboard_writer_for_test` does **not** capture board copies.

**Where in-TUI drag selection exists:**
- agent scrollback (including inside mux agent panes, because mouse events go to the focused pane: `views/navigator_mux_routing.rs:74-81`);
- the agent composer;
- the turn content modal;
- the board details strip.

**Where it does not exist:**
- every dialog (Error, Notification, SessionStorage, BoardCard, worktrees, checkpoint modals, help, daemon, add-workspace, role);
- the full-screen views: blocklist, changed files, checkpoints, model selector, resume and search;
- the agent header and footer, and the HITL, pause and exec-stdin prompt headers.

**Native terminal selection is blocked everywhere.**
- Mouse capture is switched on at startup (`terminal/mod.rs:103`) and stays on for the whole session.
- The "release capture for native selection" toggle, `mouse/toggle.rs` `MouseTrackingToggle`, is never constructed. Its `Action::ReEnableMouseTracking` arm is empty (`app/dispatch.rs:155-157`). See ST-2.
- The only fallback is the terminal's Shift+drag bypass. That copies screen cells, including borders and `…`-truncated text.

**Browser launcher (for the parent's `OAuthOpenUrl` fix):**
- `app/board_card_attachments.rs:27` `AttachmentLauncher = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>`.
- Production is `open::that(url)` (:80), run with `spawn_blocking` (:223).
- The test seam is `App::set_board_attachment_launcher_for_test` (:159).

**Wrap helpers to reuse:**
- `views/agent/text_wrap.rs:29` `wrap_to_width(text, width) -> Vec<String>`. It measures display width, honours `\n` hard breaks and slices long tokens such as URLs.
- `components/dialog_theme_rows.rs:43` `wrap_row_count`.
- The "fill the screen, wrap, scroll" dialog pattern in `components/session_storage_dialog.rs:97-126`: `help_dialog_scroll::fill_rect`, then `body_content_rows`, then `wrap_to_width`, then `render_dialog_at`, then `render_list_scrollbar`.
- `components/daemon_form_dialog.rs:46` `scroll_window(chars, cursor, width)`, a horizontally scrolling one-line input, and `:67` `wrap_text` (counts chars, not width).
- `components/board_card/wrap.rs:7` `wrap` (width-aware).

**Test harness (rust/fspec-tui/tests):**
- `common::MockBackend` with `App::new(Arc<dyn FspecBackend>)`, or `common::harness::AppTestHarness::new()`, which seeds session `s-1` in the Agent view.
- Clipboard: `App::set_clipboard_writer_for_test(Box::new(SharedWriter(Arc<Mutex<Vec<u8>>>)))`. Then drain with `while let Some(a) = app.try_recv_action() { app.dispatch(a) }` and compare against `osc52(text)`. Copy this pattern from `tests/provider_settings_input_copy_prov138.rs:109-160`.
- Events go through `App::handle_event(&Event)` (`app/composer_draft.rs:97`) and `App::handle_paste` (:105).
- Render: `common/text_selection_anchor_copy010_helpers.rs:76` `render_app(app, w, h)` (TestBackend), plus `mouse(kind, col, row)` and `seed_line`.
- Dialog access: `app.compositor()` / `app.compositor_mut()` (`app/state/state_accessors.rs:27,32`).

---

## Card A: copy everywhere text is shown

### CP-1: The "Session error" dialog cannot be copied
- **Where it comes from.**
  - Every `Action::SessionOperationFailed` (about 45 emitters across `app/`: daemons, workspaces, checkpoints, attachments, model bootstrap, git) is collected at `app/session_storage.rs:56-74`.
  - Manifest issues land in the same dialog, with `Path:`, `Session:` and `Recovery:` fields (`components/session_storage_dialog.rs:31-41`).
- **What blocks copy.** The dialog's `handle_event` (`session_storage_dialog.rs:69-95`) consumes every key and mouse event, and only Esc and the scroll keys do anything.
- **What the user sees.** The TUI's main error surface wraps and scrolls correctly, but its full error text, file paths and recovery commands cannot be copied at all.
- **Fix.**
  - In `handle_event`, add `KeyCode::Char('c')` and Ctrl+C. Both should send `Action::CopyToClipboard(self.message.clone())` on the dialog's existing `self.sender`.
  - Add `c copy` to the footer at :119.
  - Optionally add drag selection over the wrapped rows, reusing `SelectionRecognizer` the same way `turn_modal_select.rs` does.
- **Test.**
  1. `App::new(MockBackend)`, then the clipboard sink.
  2. `app.dispatch(Action::SessionOperationFailed("x".repeat(300) + " /very/long/path"))`.
  3. Assert `app.compositor().contains(SESSION_STORAGE_DIALOG_ID)`.
  4. Send `handle_event(Key 'c')`, then drain.
  5. The clipboard bytes must equal `osc52(full_message)`, and the full message must not be truncated.

### CP-2: The "API Error" dialog cannot be copied (and is clipped, see WR-1)
- **Where.**
  - Emitted at `app/dispatch_dialog_dismiss.rs:55-60` as `ErrorDialog::new(format!("API Error: {error}"))`.
  - Key handling is `components/error_dialog.rs:63-80`: Esc only. Mouse events are ignored and fall through to the scrollback underneath.
- **What the user sees.** A red modal with a clipped error and no copy. The same text exists as a scrollback line, but that copy is broken at wrap points (CP-6).
- **Fix.**
  - Give `ErrorDialog` an `Option<UnboundedSender<Action>>` through a builder like `NotificationDialog::with_action_tx` (`notification_dialog.rs:145`). Wire it at :58.
  - Make `c` / Ctrl+C emit `CopyToClipboard(self.message)`.
  - Update the footer to "c copy · Esc dismiss".
- **Test.**
  1. `app.dispatch(Action::ChunkReceived(sid, StreamChunk::Error{ error: long }))`.
  2. Press 'c', then drain.
  3. The clipboard must equal `osc52("API Error: " + long)`.

### CP-3: The notification dialog cannot be copied
- **Where.** `components/notification_dialog.rs`. The only production use is `app/dispatch_daemon_admin.rs:138` ("Saved daemon {name}. The TUI stays connected…", with no auto-dismiss).
- **What the user sees.** No way to copy the text, and it is clipped (WR-2).
- **Fix.** Same as CP-2. `self.action_tx` already exists.
- **Test.** Dispatch the daemon-edit path through MockBackend's daemon directory, or push `NotificationDialog::info(long).with_auto_dismiss_ms(0).with_action_tx(tx)` straight into `app.compositor_mut()`. Press 'c' and assert the full message is on the clipboard.

### CP-4: The board card details dialog has no copy at all
- **Where.** `components/board_card/events.rs:51-80`. In Details mode, a left-button Down only activates links, and every event is consumed. Rendering is `board_card/frame.rs` and `board_card/page.rs`.
- **What the user sees.** The full work-unit document can only be read: description, acceptance criteria, linked ids and attachment paths. The card id and title in the border title are not selectable either.
- **Fix.** The page already holds its wrapped rows.
  - Add a `c` key that emits `CopyToClipboard` with the current page's plain text: the id and title, then the body joined by newlines.
  - Add a `y` key that copies the card id.
  - Or add drag selection by reusing `SelectionRecognizer` together with `paint_selection_highlight`.
  - Extend `FOOTER` (`frame.rs:13`) to match, which also needs WR-9.
- **Test.**
  1. Use the existing `tests/board024_selection_harness.rs` / `board024_canonical_backend` fixtures to open the details for a card.
  2. Press 'c', then drain.
  3. The clipboard must contain the full description, including text below the fold.

### CP-5: The board details strip copies only the truncated on-screen rows
- **Where.**
  - `views/board/details_select.rs:117-129` and `reconstruct_strip_text` (:165-182).
  - These use `details_strip.rs:140-152` `visible_strip_rows`: the title is cut with `…` by `truncate_to` (:229), and the description is squeezed into two lines by `wrap_to_two_lines` (:184).
- **What the user sees.** Selecting the description copies at most two lines that end mid-word, and a long title copies with a `…` on the end. This is by design under COPY-009, but it is a copy-fidelity gap.
- **Fix.** On a whole-line selection (long press) of row 0, 1 or 2, copy the full source field (`unit.id: unit.title`, or the full normalized `unit.description`). Also route through `Action::CopyToClipboard` instead of `BoardView.clipboard` (see CP-14).
- **Test.**
  1. Use the `tests/common/board_details_strip_copy009_helpers.rs` `render` helper with a unit whose description is 500 chars.
  2. Long-press row 1 (`poll_selection_tick_after_for_test`).
  3. The clipboard must equal the full description.

### CP-6: Scrollback and turn-modal copies insert newlines at soft-wrap points and stop at the viewport
- **Where.**
  - `views/agent/scrollback_copy.rs:30-44` joins wrapped visual rows with `"\n"`. The rows come from `store/agent_view/chunk_wrap.rs`, which calls `wrap_to_width`.
  - `views/agent/turn_modal_select.rs:197-213` behaves the same way.
  - Any scroll clears the selection (`app/dispatch_scroll.rs:13`), so a selection can never be longer than the viewport.
- **What the user sees.** A 150-char URL, path or command copied from a notice, tool output or the `API Error:` line pastes with hard line breaks in it, so it is unusable in a shell or a browser. Long tool output cannot be copied in one go.
- **Fix.**
  1. Record soft-wrap continuation per visual row (for example, a `continues: Vec<bool>` beside `RenderedChunk::lines`, set in `chunk_wrap`), and join continuation rows without `\n` in both reconstructors.
  2. Add a "copy whole turn" key: `c` in SELECT mode (`views/agent/dispatch_select.rs`) and in the open turn modal. It emits `CopyToClipboard(scrollback.full_text_for_seq(seq))`, which already exists at `views/agent/scrollback_select.rs:117`. Add it to `MODAL_FOOTER` (`turn_modal.rs:37`).
- **Test.**
  1. `AppTestHarness::new()`.
  2. `app.dispatch(Action::ChunkReceived(s1, Text(url_200_chars)))`.
  3. `render_app(&mut app, 80, 24)`.
  4. Drag from the URL's first cell to its last cell (`mouse(Down/Drag/Up, ..)`), then drain.
  5. The clipboard must equal the URL, with no `\n`.
  6. Second case: Tab, Enter (open the modal), 'c'. The clipboard must equal `full_text_for_seq`.

### CP-7: Changed files and checkpoints cannot copy a path or a diff
- **Where.**
  - `views/changed_files/mod.rs:342` `handle_key` has no `Char` keys.
  - `views/checkpoints/keys.rs` is the same.
  - Rows use `diff_common/row.rs:54` `file_row`, which shows a truncated path.
- **What the user sees.** No way to get the full path of a changed file, a checkpoint name or the diff text out of the TUI. The visible path is also cut (WR-6).
- **Fix.** `c` copies the selected file's full `ChangedFile::path`, and `C` copies `self.diff_lines.join("\n")`. In the checkpoints view, `y` copies the checkpoint name. Emit these through the view's existing Emit path into `Action::CopyToClipboard`.
- **Test.**
  1. Script the MockBackend so `changed_files` returns a 120-char path.
  2. Open the view (the existing `tests/changed_files*` setup), press 'c', then drain.
  3. The clipboard must equal `osc52(full_path)`.

### CP-8: Session worktree paths and ids cannot be copied
- **Where.** `views/agent/session_worktrees_dialog.rs:182-208`. The rows are `selectable: false`, and only Tab, the arrows, Enter and Esc are handled in `session_worktrees_dialog_dispatch.rs`.
- **What the user sees.** The `/worktrees` list shows the worktree path a user would `cd` into, but it cannot be copied and it is clipped (WR-4).
- **Fix.** Make the rows navigable with ↑ and ↓. `c` copies the highlighted `worktree_path`.
- **Test.**
  1. `MockBackend.list_session_worktrees` returns 2 rows with 90-char paths.
  2. Run the `/worktrees` slash dispatch, press ↓ then 'c', then drain.
  3. The clipboard must equal the second row's path.

### CP-9: The model selector cannot copy a model id, and paste is dropped in the custom-model form
- **Where.**
  - Keys: `views/model_selector/dispatch.rs:34-65` handles `/`, `r`, `a`, `e` and `d`. There is no copy key.
  - Paste: `views/navigator_events.rs` `handle_model_selector_event` (around :96-105) returns `ignored` for `Event::Paste`, so it never reaches the form. Compare provider settings at :38.
- **What the user sees.** The user cannot copy `provider/model-id` for a config file, and cannot paste a long model id or base URL into the add/edit custom-model form.
- **Fix.** Add `y` (or `c` when not in the form) to emit `CopyToClipboard(format!("{provider}/{model_id}"))` for the focused row. Add `ModelSelectorView::handle_paste`, mirroring `provider_settings/profile_form_paste.rs`.
- **Test.** Use the `tests/model_selector*` MockBackend `list_providers` fixture. Focus a model row, press 'y', then drain, and assert the clipboard bytes. For paste: `app.handle_paste("gpt-x")` while the form is open, then assert the form field value.

### CP-10: Blocklist rule details cannot be copied
- **Where.** `views/blocklist/mod.rs:218-219`: every `Char` other than space is swallowed. The render is `views/blocklist/panes.rs:150-228`.
- **What the user sees.** The rule ID, Pattern (regex), Reason and Guidance cannot be copied. They are also clipped (WR-5).
- **Fix.** `c` copies the focused rule's `pattern`, and `C` copies the whole detail block.
- **Test.** Open the blocklist view with the `tests/blocklist_rpc056` fixtures, press 'c', then drain, and assert the clipboard equals the pattern.

### CP-11: The attachment and FOUNDATION viewer URL is never shown
- **Where.** `app/board_card_attachments.rs:222-226` launches `open::that(url)`. On failure the error is `"browser launch failed: {error}"` and **omits the URL**. It reaches the user through `dispatch_viewer.rs:50,95` as `SessionOperationFailed`.
- **What the user sees.** Over SSH or on a headless host, D or A says "could not be opened: browser launch failed: …", but there is no URL to open by hand.
- **Fix.** Include the URL in the error, e.g. `browser launch failed for {url}: {error}`. With CP-1 in place that text becomes copyable. Optionally, always emit a notice with the URL on success too.
- **Test.**
  1. `app.set_board_attachment_launcher_for_test(Arc::new(|_| Err("no display".into())))`.
  2. Dispatch `OpenFoundation` with a MockBackend workspace.
  3. Await the pending task, then drain.
  4. The session-storage errors must contain `"http://127.0.0.1:"`.

### CP-12: The merge success notice with the commit sha disappears immediately
- **Where.** `app/dispatch_merge_worktree.rs:219-227`. It sends `EmitSessionNotice("[merge] success (<sha>)")` and then straight away `MergeSuccess`, which tears down the session and returns to the board (`app/merge_worktree_success.rs`).
- **What the user sees.** The merge commit sha is never readable or copyable.
- **Fix.** Surface the outcome where it survives, such as a board-level `NotificationDialog::success(..)` or a board status line.
- **Test.**
  1. MockBackend `merge_session_worktree` returns `Success { merge_commit: Some("abc123") }`.
  2. Drain.
  3. Assert that a surviving surface (a compositor dialog or a board store field) contains "abc123".

### CP-13: HITL, pause and exec-stdin prompt headers are not selectable (verify)
- **Where.** `views/agent/mouse_dispatch.rs:255-280`. `handle_composer_mouse` passes clicks inside `last_input_area` to `self.input.handle_mouse`, which maps them to the input body. The headers painted above it are not mapped: `hitl_prompt.rs:148`, `pause_prompt.rs:144` and `stdin_answer_box.rs:150`.
- **What the user sees.** The agent's question text and the waiting command's name cannot be copied.
- **Fix.** Low priority. When a prompt is active, a `c` (or Ctrl+Shift+C) copies the header text.
- **Test.** Render with an active HITL request, drag over the header rows, and assert what is (or is not) copied. Do this first to confirm the gap.

### CP-14: There is no copy confirmation, and copy goes through split writers
- **Where.**
  - `app/dispatch_scroll.rs:234-241` and `details_select.rs:124` only log on error. Nothing tells the user that a copy happened.
  - The board uses its own `Osc52Clipboard` (`views/board.rs:107`), and agent selection writes `self.clipboard` directly (`dispatch_scroll.rs:225`) instead of going through `CopyToClipboard`.
- **What the user sees.** Every copy is silent. On terminals that ignore OSC 52 (tmux without `set-clipboard on`, some IDE terminals) it looks dead. That is the same symptom as the reported OAuth bug, even after the parent's fix.
- **Fix.**
  1. Route all copies through `Action::CopyToClipboard`, removing `BoardView.clipboard`.
  2. In `handle_copy_to_clipboard`, show a short confirmation such as "Copied N chars". Options: a short `NotificationDialog::success(..).with_action_tx(tx)` (2000 ms default), or a session notice or view status line.
- **Test.** Copy from the board strip, then assert `App`'s injected sink received the bytes (today it receives nothing). After any copy, assert the confirmation is present in the compositor or status.

---

## Card B: wrap long text

General root cause:
- `components/dialog_theme.rs:95-140` sizes every shrink-to-content dialog as `min(content + 4, area.width)`. The body width is that minus 4.
- `dialog_theme_rows.rs:157` `paint_left_aligned` paints each `DialogRow` on exactly **one** line, one char per cell, and stops at the edge.
- Visible chars per row: **76 at 80 columns, 116 at 120 columns** (the area width minus 4).
- `sanitize_for_terminal` keeps `\n`, so a multi-line message paints a raw `"\n"` cell symbol, which corrupts the terminal output.

**Root fix.** Add a wrapping row builder in `dialog_theme_rows.rs`, for example `wrapped_rows(text, style, max_width) -> Vec<DialogRow>`:
- Split on `\n` and run `wrap_to_width`.
- Take the max width from the area: `area.width.saturating_sub(8)`.
- For unbounded text, use the SessionStorageDialog fill-and-scroll pattern.

### WR-1: The ErrorDialog message is clipped, and newlines break the display
- **Where.** `components/error_dialog.rs:82-98`, through `render_dialog`.
- **What the user sees.** `API Error: <provider body>` shows 65 chars of the error at 80 columns and 105 at 120. A JSON error body that contains `\n` corrupts the screen.
- **Fix.** Use `wrapped_rows`. If there are more rows than fit, switch to `fill_rect` with scrolling as in `session_storage_dialog.rs`.
- **Test.** `render_app` at 80x24 with a 300-char error that contains `\n`. Every buffer cell must have a symbol without `\n`. Every 76-char slice of the error must appear in the buffer rows. Repeat at 120 columns.

### WR-2: The notification dialog is clipped
- **Where.** `components/notification_dialog.rs` render (through `render_dialog`). The message comes from `dispatch_daemon_admin.rs:138`: 124 chars plus twice the daemon name.
- **What the user sees.** Clipped at both 80 and 120 columns, so the end of the sentence, "…the next time it connects to {name}.", is lost.
- **Fix.** `wrapped_rows`.
- **Test.** Push the dialog and render at 80 and at 120. The buffer must contain "connects to".

### WR-3: Checkpoint restore and delete confirmations and errors are clipped
- **Where.**
  - `views/checkpoints/dialog.rs:74-91`: `"Restore {path}? This overwrites the working copy."` and `"Error: {message}"`.
  - `views/checkpoints/delete_dialog.rs:120`.
  - Both are painted by `components/checkpoint_restore_dialog.rs:20` (one row per line).
- **What the user sees.** With a path longer than about 28 chars at 80 columns, or about 68 at 120, the user confirms a destructive restore without seeing the question mark or the "overwrites" warning. Restore errors are clipped.
- **Fix.** `wrapped_rows` in `render_restore_modal`.
- **Test.** Open the restore confirmation on a file with a 100-char path and render at 80. The buffer must contain "overwrites the working copy".

### WR-4: The session worktrees dialog clips its rows and does not scroll
- **Where.** `views/agent/session_worktrees_dialog.rs:182-208, 236-260`. Each row is `"[dirty] " + session_id (36-char UUID) + "  " + worktree_path`, where the path is about `<repo>/.fspec/worktrees/<uuid>` and roughly 80 chars or more.
- **What the user sees.** A row of about 126 chars or more is clipped at both 80 columns (about 30 chars of the path visible) and 120. With many worktrees the dialog height is clamped and the extra rows, and possibly the Prune/Cancel buttons, are lost.
- **Fix.** Render the path on its own wrapped second line per worktree (`wrapped_rows`), and add scrolling (fill_rect pattern) once the rows exceed the height.
- **Test.** Use a MockBackend with 3 worktrees whose paths are 90 chars, and render at 80x24. Each full path must be recoverable from consecutive buffer rows, and the "Prune" button must be visible. Then use 40 worktrees at 80x24: the buttons must still be visible.

### WR-5: The blocklist detail pane does not wrap
- **Where.** `views/blocklist/panes.rs:150-228` (`Paragraph::new(lines)` with no `.wrap`). The pane is 50% of the body width (`blocklist/render.rs:63-65`).
- **What the user sees.** At 80 columns the pane is about 39 columns, and about 59 at 120, so Pattern, Reason and Guidance sentences are cut off.
- **Fix.** `Paragraph::new(lines).wrap(Wrap { trim: false })`, or pre-wrap with `wrap_to_width` so the scroll math stays exact.
- **Test.** Use a fixture rule whose guidance is 200 chars and render at 80x24. The last word of the guidance must appear in the buffer.

### WR-6: Changed-file and checkpoint paths lose the file name, and diff lines are clipped
- **Path truncation.** `views/diff_common/row.rs:33-47` `truncate_path` keeps the **head** and cuts the tail, so `rust/fspec-tui/src/views/prov…` loses the file name.
  - The files pane is 40% of the width (`changed_files/render.rs:83-85`), so the path budget is about 28 chars at 80 columns and about 44 at 120.
  - The checkpoint label has the same problem (`checkpoints/render.rs:194-197`).
- **Diff lines.** `changed_files/render.rs:161-186` and the checkpoints diff pane render a `Paragraph` with no wrap and no horizontal scroll. The diff pane is 60%, about 47 columns at 80, and anything beyond that is invisible.
- **Fix.**
  - Middle- or left-ellipsize the path so the file name is kept, e.g. `…/views/provider_settings/oauth_login.rs`.
  - For diffs, add ←/→ horizontal scroll in the Diff pane, or soft-wrap with `wrap_to_width`. Keep the scroll math in one place, like `turn_modal_geometry`.
- **Test.** Render at 80 with the path `"a/".repeat(30) + "file_name.rs"`. The buffer must contain "file_name.rs". Use a diff line of 120 chars: its tail must be visible after wrapping or after pressing →.

### WR-7: The exec-stdin notice and answer box hide their own key hints
- **Where.** `views/agent/stdin_answer_box.rs:48-60` (`command_line`), :100-110 (`paint_notice`), :150 (the box header), and :175 (the send error). Each is a one-row `Paragraph`.
  - `NOTICE_TAIL` is 58 chars: " may be waiting for input · Ctrl+O answer · Ctrl+X dismiss".
  - The BUG-364 stale variant adds "may be stale · probe failed: {error}" **before** the keys (:83-94).
- **What the user sees.** At 80 columns, a command longer than about 20 chars pushes "Ctrl+O answer · Ctrl+X dismiss" off screen, and a probe error always does. The send error is clipped too.
- **Fix.** Put the key hints first or on their own row, wrap the command and the error with `wrap_to_width`, and count the extra rows in `focused_stdin_box_height`.
- **Test.** Use a MockBackend exec-stdin request with a 70-char command and render at 80. The buffer must contain "Ctrl+X dismiss". Repeat with `probe_error = Some(long)`.

### WR-8: The model selector write error is on one row
- **Where.** `views/model_selector/render.rs:108-118` paints `"[error] {error}"` on one row.
- **What the user sees.** The BUG-325 custom-model write failure is clipped at the screen width.
- **Fix.** Wrap it to at most N rows (`wrap_to_width`) and reserve that many rows from `body_area`.
- **Test.** Dispatch `CustomModelWriteFailed{message: 200 chars}` and render at 80. The tail of the message must appear.

### WR-9: The board card details status row and footer are truncated
- **Where.**
  - `components/board_card/frame.rs:13-14`: `FOOTER` is **100 columns**, painted with `set_stringn` at the inner width (the terminal width minus 2). It is cut below 102 columns. At 80 columns it ends at "…│ ↑↓ PgUp P", so "Home End wheel scroll" is lost.
  - `frame.rs:46-56`: the attachment error `"Error: {path}: {message}"` is one clipped row.
- **Fix.** Wrap the footer into two rows when it does not fit (the inner height already reserves 2 chrome rows; make that dynamic). Wrap the status text with `board_card::wrap::wrap`.
- **Test.** Render the details at 80x24. The buffer must contain "wheel scroll". Inject `BoardCardAttachmentFailed{ path: 60 chars, message: 80 chars }`: the message tail must be visible.

### WR-10: The add-workspace and role dialogs use one-row drafts and errors
- **Where.**
  - `components/add_workspace_dialog.rs:100-130`: the draft is a raw `Span`, and the `{error:#}` anyhow chain comes from `app/dispatch_workspace_admin.rs:46`.
  - `components/role_dialog.rs:195-225`.
- **What the user sees.** A long typed path grows the dialog and is then clipped, so the cursor end is invisible. The full add error chain is clipped.
- **Fix.** Reuse `daemon_form_dialog::scroll_window` for the draft and `wrap_text`/`wrap_to_width` for the error, which is exactly what `DaemonFormDialog::error_rows` (:197) does.
- **Test.** `app.handle_paste(200-char path)` into the open AddWorkspaceDialog and render at 80. The buffer must contain the path's last 20 chars.

### WR-11: Full-screen footer hints are on one row (low)
- **Where.** `views/agent/mode_view_render.rs:84` `render_footer_hint` (`Paragraph`, no wrap).
- **What the user sees.** The blocklist footer is exactly 80 columns (`blocklist/render.rs:25`) and is clipped in mux panes or on narrower terminals. Other views' footers are 45-74 columns and clip inside mux panes.
- **Fix.** Fall back to a shorter variant or two lines when the hint is wider than the area.
- **Test.** Render the blocklist at 79 columns. The buffer must contain "Esc".

---

## Card C: UI-wired features backed by stubs

### ST-1: The WebSocket transport forwards no OAuth calls
- **Where.**
  - The trait defaults in `transport/backend/credentials.rs:63-130` return `"headless OAuth login is not supported"`, `"device OAuth login is not supported"`, or `Unimplemented`.
  - Overridden only by `transport/embedded/provider_auth.rs`. `WebSocketFspecBackend` (`transport/websocket/*.rs`) implements none of `oauth_headless_start`, `oauth_headless_complete`, `oauth_device_start`, `oauth_device_poll`, `oauth_copilot_device_start`, `oauth_clear_tokens` or `oauth_get_tokens`.
  - The RPC service **does** declare them (`rust/rpc/src/service_declaration/supervision_credentials.rs:140-180`).
  - The UI hides only the *browser* rows when the transport cannot run browser login (`app/dispatch_provider_settings.rs:32`).
- **What the user sees.** In client mode (`fspec` attaching to a daemon: `rust/fspec/src/client.rs:59`), and after any daemon switch:
  - the headless "sign in with code", device and Copilot login rows are shown but always land on "OAuth Login error: headless OAuth login is not supported";
  - Logout always shows "✗ OAuth logout failed; credentials may remain. Retry after checking storage access." (`app/oauth_disconnect.rs:13-18`), which is misleading.
- **Fix.** Add one-line forwards in the WebSocket backend: `client.read().await` guard, then `client().oauth_*(context::current(), ..)`, the same shape as `websocket/session_context.rs:88-120`. If any must stay unsupported, hide those rows the same way `supports_browser_oauth` hides browser rows.
- **Test.** This needs a WebSocket integration harness (`rust/fspec/tests/client_mode.rs` style), so it is not a pure App-dispatch test. At App level: give MockBackend `oauth_headless_start → Err("headless OAuth login is not supported")` and assert that the row is hidden, or that the error is accurate.

### ST-2: The native-selection toggle is scaffolding only, so mouse capture is always on
- **Where.**
  - `mouse/toggle.rs` (`MouseTrackingToggle`, "scaffolding for RPC-019") is never constructed.
  - The `Action::ReEnableMouseTracking(_owner)` arm is empty (`app/dispatch.rs:155-157`). The comment admits "does not yet route it" (`components/action_board.rs:76-84`).
  - Capture is enabled once at startup (`terminal/mod.rs:103`).
- **What the user sees.** In every view without in-TUI selection (Card A list), dragging does nothing. Terminal-native selection needs Shift+drag (it depends on the terminal), and that copies screen cells including borders and `…`.
- **Fix.** Either wire the toggle (for example, a key that releases capture for 5 seconds), or delete the scaffolding and rely on the Card A copy keys.
- **Test.** If wired: the key emits the toggle, `MouseTrackingToggle::new(Vec<u8>, ..)` receives the `DisableMouseCapture` bytes, and `ReEnableMouseTracking` re-enables capture.

### ST-3: The WebSocket transport has no checkpoint-progress or git-state push (low, documented)
- **Where.** `transport/backend/subscriptions.rs:87-107`: the default receivers are closed. Only embedded overrides them.
- **What the user sees.** In client mode the TUI-107 loading dialog never shows `(idx/total)`, and git state relies on the poll fallback.
- **Fix.** Forward the daemon's subscriptions if the RPC exposes them.
- **Test.** MockBackend with a closed receiver: the loading dialog shows `(idx/…)`. This documents the behaviour; it is not a bug test.

### ST-4: Informational only (no user-visible feature affected)
- `components/status_dialog.rs` `StatusDialog` is used only in tests.
- `rust/tui`:
  - `StatusDisplay` ("ESC to interrupt", `tui/src/status.rs:31`), `create_event_stream` (`tui/src/events.rs:25`) and `restore_terminal`/`set_panic_hook` (`tui/src/terminal.rs`) have no callers outside the crate.
  - Only `InputQueue` is used, by `cli/src/interactive/stream_loop/{driver,state}.rs` and `gemini_continuation*.rs`.
- `rust/cli/src/interactive` has no `KeyCode` handling, raw mode, alt screen or mouse capture. It writes plain stdout, so native terminal selection works and there are no copy or wrap gaps there.
- Outside this slice (provider_settings, for the parent): `views/provider_settings/detail.rs:268` shows "{provider_id} uses OAuth which is not yet supported in the Rust frontend".
