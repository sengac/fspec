# PROV-137 — Paste support for `/provider` view input areas

**Type:** Bug · **Epic:** provider-settings-parity · **Surfaces:** Provider Settings profile form + inline API-key entry
**Blocks:** PROV-138 (copy support)

---

## 1. Problem statement

In the Rust `/provider` (Provider Settings) view, the text-entry areas do **not** accept paste:

- **Profile create/edit form** — fields: `Name`, `Base URL`, `API Key`, `Context Window`, `Max Output Tokens`, `Compaction Threshold`.
- **Inline API-key entry** — the `EditApiKey` draft on a provider Detail row.

The terminal is started with **bracketed paste enabled**, so crossterm delivers the whole pasted blob as a single `Event::Paste(String)` — NOT as a stream of `KeyCode::Char` events. That single event is currently **dropped** before it reaches the view.

### Security requirement (user-mandated)

The **API-key field must remain masked after a paste**. The pasted secret is stored in the draft buffer (so it can be saved), but the on-screen render continues to show bullet dots `•` — the plaintext secret is never displayed. This already holds for typed input (see `detail.rs:231` render) and must continue to hold for pasted input — i.e. paste must NOT introduce any new plaintext render path.

---

## 2. Root cause — where the paste event dies

Event flow for a paste:

1. `src/terminal.rs:98` — `EnableBracketedPaste` in the terminal setup plan → crossterm emits `Event::Paste(String)`.
2. `src/app/events.rs:224-226` — the run loop's `Event::Paste` arm calls `App::handle_paste`.
3. `src/app/events.rs:162-177` — `handle_paste` routes: **Compositor first** (`compositor.handle_paste`, compositor.rs:197), then falls through to **`navigator.handle_event(&Event::Paste(...))`**.
4. `src/views/navigator.rs:101-111` — `handle_event` dispatches by `active_view`; for the provider view → `handle_provider_settings_event`.
5. **`src/views/navigator_events.rs:26-37`** — THE GAP. This handler only matches `Event::Mouse` and `Event::Key`:
   ```rust
   if let Event::Mouse(mouse) = event { ... }
   let Event::Key(key) = event else {
       return EventResult::ignored();   // <-- Event::Paste dies here
   };
   ```
   So `Event::Paste` returns `Ignored` and never reaches `ProviderSettingsView`.

`ProviderSettingsView::handle_key` (`mod.rs:188`) and its sub-handlers only accept per-char `KeyCode::Char(c)`:
- API key: `detail.rs:146-157` (`handle_edit_key`, gated by `is_printable_ascii`, 32..=126).
- Profile form: `profile_form.rs:197` (`route_key` → `push_char`, gated `(' '..='~')`).

---

## 3. The established paste-sink pattern (reuse, do not reinvent)

Three existing sinks already consume `Event::Paste` correctly. Copy this pattern:

- **Agent composer** — `multiline_input.rs:290` → `multiline_input_paste.rs:21` (`handle_paste` → `insert_str(normalize_line_endings(text))`).
- **Role dialog** — `components/role_dialog.rs:155-162` matches `Event::Paste(s)` in `handle_event`, inserts `normalize_line_endings(s)`.
- **HITL freeform prompt** — `views/agent/hitl_keys.rs:181-197`.

Shared helper: **`src/text_normalize.rs:18`** — `normalize_line_endings(text)` collapses `\r\n` and lone `\r` → `\n`.

> ⚠️ The provider fields are **single-line** (Base URL, API key, numeric fields, profile name). The agent composer is multiline and inserts newlines verbatim. For provider fields, newlines must be **stripped** (not inserted), and each pasted char must pass the field's existing charset filter.

---

## 4. Design / implementation plan

### Step 1 — Stop dropping paste at the navigator boundary
`navigator_events.rs::handle_provider_settings_event`: add an `Event::Paste(text)` arm **before** the `Event::Key` guard that forwards to a new `ProviderSettingsView::handle_paste(text)` and translates its `ProviderSettingsEvent` exactly like the key path (Consumed/Ignored/Emit/Close/SwitchToModels).

### Step 2 — `ProviderSettingsView::handle_paste(&mut self, text: &str) -> ProviderSettingsEvent`
Lives in `mod.rs`. Dispatch on `self.mode.clone()` mirroring `handle_key`, but ONLY the text-entry modes act; all others return `Ignored`:
- `CreateProfile { provider_id, form }` / `EditProfile { provider_id, profile_name, form }` → `profile_form::handle_form_paste(...)`.
- `Detail { provider_id, sub: EditApiKey { draft } }` → `detail::handle_edit_paste(...)`.
- everything else → `ProviderSettingsEvent::Ignored`.

### Step 3 — Field-level insertion (charset + newline filtering)
- **Profile form** (`profile_form.rs`): add `ProfileForm::insert_str(&mut self, text: &str)` that iterates chars, and for each char passing the existing printable gate `(' '..='~')` calls `push_char(c)` (which already routes to the name when `is_editing_name`, else the focused field). Newlines/tabs/control chars are dropped (they fail the gate). Then persist the mutated form back into the mode (reuse `restore_mode`).
- **API key** (`detail.rs`): add a paste handler that appends each `is_printable_ascii(c)` char of `text` to `draft`, drops the rest (newlines included), and re-stores `DetailSub::EditApiKey { draft }`. **Do not touch the renderer** — `detail.rs:231` already renders `"•".repeat(draft.len())`, so the pasted secret stays masked automatically.

### Step 4 — Masking invariant
No new render path is added for the API key. The draft holds the true bytes (needed for Save); the render is bullets. Add a test asserting that after pasting an API key, the rendered line contains only `•` (no plaintext substring of the pasted secret).

---

## 5. Example Mapping seed

**Rules**
1. Pasting into the profile form's focused field inserts the pasted text at the field.
2. Pasting into the inline API-key entry appends the pasted text to the key draft.
3. Newlines and control characters in a paste are stripped (single-line fields).
4. Only the field's allowed charset survives a paste (printable ASCII 32..=126 for API key; `(' '..='~')` for form fields).
5. After pasting an API key, the field stays masked — the render shows only `•`, never the plaintext.
6. Pasting while a non-input mode is focused (List, Detail summary, OAuth) is a no-op.

**Examples**
- Focus Base URL, paste `https://api.example.com`, the field shows that URL.
- Focus API Key in the profile form, paste `sk-secret123`, the field shows `••••••••••••` (12 dots), and Save stores `sk-secret123`.
- Inline EditApiKey, paste `sk-abc\ndef`, draft becomes `sk-abcdef` (newline stripped), render shows 9 dots.
- Paste `héllo` into Context Window (numeric) — non-ASCII `é` dropped, `hllo`? No — the form field gate is `(' '..='~')` which drops `é`; result is `hllo`. (Numeric validity is enforced later at build time, unchanged.)
- Paste while on the provider List → nothing changes.

---

## 6. Files in scope

| File | Change |
|------|--------|
| `codelet/fspec-tui/src/views/navigator_events.rs` | Add `Event::Paste` arm in `handle_provider_settings_event`. |
| `codelet/fspec-tui/src/views/provider_settings/mod.rs` | Add `handle_paste` dispatch. |
| `codelet/fspec-tui/src/views/provider_settings/profile_form.rs` | Add `ProfileForm::insert_str` + `handle_form_paste`. |
| `codelet/fspec-tui/src/views/provider_settings/detail.rs` | Add `handle_edit_paste`. |
| `codelet/fspec-tui/src/text_normalize.rs` | Reuse (maybe a single-line variant that maps `\n`→drop). |

Watch the **300-LoC ceiling**: `profile_form.rs` is ~291 lines, `detail.rs` is large. Extract into a sibling module (e.g. `profile_form_paste.rs`) if needed.

---

## 7. Test guidance

Mirror existing patterns in `tests/provider_settings_api_key_charset_rpc161.rs` and `tests/provider_settings_profile_form_prov110.rs`. Drive `handle_paste` (or feed `Event::Paste` through the navigator) and assert field/draft contents and, crucially, the **masked render** for the API key. Every Gherkin step needs a matching `// @step` comment with EXACT text. Tests must fail first (red), then pass after implementation.

## 8. Non-goals
- No OSC 52 clipboard READ (that's a terminal limitation; paste comes via bracketed paste, not clipboard reads).
- Copy-out (Ctrl+C) is PROV-138, not this card.
- model_selector custom-model form paste is out of scope here (separate surface).
