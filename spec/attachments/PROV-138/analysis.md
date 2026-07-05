# PROV-138 — Copy support for `/provider` view input areas (Ctrl+C, API key masked)

**Type:** Bug · **Epic:** provider-settings-parity · **Surfaces:** Provider Settings profile form + inline API-key entry
**Depends on:** PROV-137 (paste — shared `Event`/view plumbing)

---

## 1. Problem statement

In the Rust `/provider` view input areas there is no way to copy the current value of the focused field to the system clipboard. This card adds **Ctrl+C** to copy the focused field's value via the existing OSC 52 clipboard writer.

### Security requirement (user-mandated)

Copying the **API-key field must copy the MASKED value** (bullet dots `•`), matching what's on screen — the plaintext secret must never leave the field through the clipboard. This applies to BOTH:
- the profile form's `API Key` field (field index 1), and
- the inline `EditApiKey` draft on a provider Detail row.

All other fields (`Name`, `Base URL`, `Context Window`, `Max Output Tokens`, `Compaction Threshold`) copy their plaintext value.

---

## 2. Existing infrastructure to reuse

**OSC 52 clipboard writer (COPY-001)** — `src/mouse/clipboard.rs`:
- `Osc52Clipboard::with_stdout()` — production writer to real stdout.
- `Osc52Clipboard::new(writer)` — generic over `W: Write + Send` so tests inject a `Vec<u8>` and assert exact bytes.
- `copy(&mut self, text: &str) -> io::Result<()>` — emits `ESC ] 52 ; c ; base64(text) BEL`.

This is the SAME writer the COPY-006…011 text-selection features use (AgentView scrollback, turn modal, board strip, composer). Precedent for a Ctrl+C style copy chord + injected test writer already exists across those cards — follow it.

The provider settings view also already has a precedent for a clipboard action: `oauth_login.rs:116` — `c` copies the authorize URL. Check how that reaches the clipboard for the established provider-view wiring (it may go through an `Action`).

---

## 3. Design decisions — RESOLVED via DeepSearch

**Dominant pattern (confirmed): Action-emit → App reducer → `self.clipboard.copy`.** Views NEVER call `Osc52Clipboard` directly.

- `Action::CopyToClipboard(String)` ALREADY EXISTS (`components/mod.rs:1168`) and ALREADY dispatches: `dispatch.rs:251` → `App::handle_copy_to_clipboard` (`dispatch_scroll.rs:225-232`) → `self.clipboard.copy(&text)` → `Osc52Clipboard::copy` (`mouse/clipboard.rs:48`).
- The App's `clipboard` field lives on `App` (`app/state.rs:82`, constructed at `:125`). Test seam: `App::set_clipboard_writer_for_test` (`app/dispatch_scroll.rs:239-244`, NOT `#[cfg(test)]` — usable from integration tests via an injected `Vec<u8>`/`Box<dyn Write+Send>`).
- **Implication: ZERO new App wiring.** The provider input handler just returns `ProviderSettingsEvent::Emit(Action::CopyToClipboard(text))`. The masking transform (for the API key) happens in the VIEW before building the action, so `handle_copy_to_clipboard` receives already-masked text.
- Precedent for the exact pattern: COPY-007 composer copy emits `Action::CopyToClipboard(text)` from `views/agent/mouse_dispatch.rs:173`.
- ⚠️ NOTE: the provider `oauth_login.rs` 'c' (copy authorize URL) emits `Action::OAuthCopyUrl` which is currently a **log-only stub** (`dispatch_provider_settings_oauth.rs:82-84`) — do NOT copy that broken path; use `Action::CopyToClipboard`.

The masking transform must happen BEFORE the action is built, so the plaintext secret never enters the action bus.

2. **Masked value format:** the copied API-key text must equal the on-screen mask, i.e. `"•".repeat(value.chars().count())` — matching `detail.rs:231` (`"•".repeat(draft.len())`). Confirm char-count vs byte-len consistency with the renderer and keep them identical (single source of truth — consider a shared `mask_secret(&str) -> String` helper reused by both render and copy).

3. **Ctrl+C vs existing bindings:** ensure Ctrl+C in these input modes doesn't collide with any global quit/interrupt. `mod.rs:192-197` currently *consumes* any CONTROL/ALT-modified key in the provider view (returns `Consumed` no-op). The copy chord must be handled BEFORE that blanket consume, only in the two input modes.

---

## 4. Example Mapping seed

**Rules**
1. Ctrl+C in the profile form copies the focused field's current value to the clipboard.
2. Ctrl+C in the inline API-key entry copies the key draft to the clipboard.
3. Copying the API-key field copies the MASKED value (`•` dots), never the plaintext secret.
4. Copying a non-secret field copies its plaintext value.
5. Copying an empty field copies an empty string (no panic).
6. Ctrl+C outside an input mode (List/Detail summary) does not copy a field value.

**Examples**
- Focus Base URL = `https://api.example.com`, press Ctrl+C → clipboard receives `https://api.example.com`.
- Focus profile-form API Key = `sk-secret123`, press Ctrl+C → clipboard receives `••••••••••••` (12 dots), NOT `sk-secret123`.
- Inline EditApiKey draft = `sk-abcdef`, press Ctrl+C → clipboard receives `•••••••••` (9 dots).
- Focus empty Context Window, press Ctrl+C → clipboard receives `""`.
- On the provider List, press Ctrl+C → no clipboard write for a field.

---

## 5. Files in scope

| File | Change |
|------|--------|
| `codelet/fspec-tui/src/views/provider_settings/mod.rs` | Intercept Ctrl+C in input modes before the blanket CONTROL consume; route to copy. |
| `codelet/fspec-tui/src/views/provider_settings/profile_form.rs` | Compute focused field value; mask when field idx == 1 (API Key). |
| `codelet/fspec-tui/src/views/provider_settings/detail.rs` | Copy masked EditApiKey draft. |
| `codelet/fspec-tui/src/mouse/clipboard.rs` | Reuse `Osc52Clipboard` (no change expected). |
| (maybe) `components/mod.rs` + App dispatch | New `Action::CopyToClipboard` if option (a) is chosen. |
| (maybe) a shared `mask_secret` helper | Single source of truth for `•` masking (render + copy). |

Mind the **300-LoC ceiling** — extract a `*_copy.rs` sibling if a file would exceed it.

---

## 6. Test guidance

- Inject a `Vec<u8>` OSC 52 writer (as COPY-001/007 tests do) and assert the EXACT bytes: for the API key, assert the payload base64-decodes to `•` dots, and assert it does NOT contain the plaintext secret substring.
- Assert masked copy uses the same length as the on-screen render.
- Every Gherkin step → EXACT `// @step` comment. Red first, then green.

## 7. Non-goals
- In-field text SELECTION (partial copy) — copy is whole-field only.
- model_selector custom-model form copy (separate surface).
- Copy of masked provider-list rows (those are already display-only masked strings).
