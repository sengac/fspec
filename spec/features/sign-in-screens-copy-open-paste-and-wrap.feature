@BUG-582
@done
@provider-settings
@tui
@providers
@oauth
@TUI-126
Feature: Sign-in screens copy, open, paste and wrap their text
  """
  Implementation:
  - shared pieces for TUI-130/131: components/copy_notice.rs (CopyNotice: App-level confirmation painted on the bottom row for 2 s on tokio time, set by App::handle_copy_to_clipboard for every CopyToClipboard; the run loop keeps drawing while it shows and once after it ends); components/wrapped_text.rs (wrapped_lines: text wrapped with views/agent/text_wrap::wrap_to_width into styled ratatui Lines, nothing cut off); app/url_launcher.rs (UrlLauncher seam shared with the board attachment launcher, App::open_url runs it on the blocking pool and reports failure as ProviderSettingsStatus). Provider Settings: OAuthHeadlessCodeEntry gains code_focused; oauth_login.rs handles URL/Code focus, c/o on the URL, printable-ASCII typing; the view emits Action::CopyToClipboard (OAuthCopyUrl removed); OAuthOpenUrl opens through the launcher; copy.rs covers the OAuth modes and a List status; paste.rs covers code entry and the GHE host; oauth_login_render.rs wraps every long value and paints the status; footer hints follow focus.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User request (2026-11-07): on Sign in with code, c copies the whole authorize URL to the clipboard through the one copy path used elsewhere (Action::CopyToClipboard, the OSC 52 writer), and o opens it in the browser through the App's URL launcher. When the browser cannot be opened the screen says why and that c copies the URL.
  #   2. BUG-634 correction: a code that starts with c or o keeps its first character after explicit Code field focus or paste. The screen opens on the URL. On the URL, c copies and o opens while retaining URL focus, including after a failed open. Any other typed character, a paste, Tab or Down moves to the Code field (the character or paste goes into the code). In the Code field every printable character, c and o included, is typed into the code; Tab or Up goes back to the URL.
  #   3. User request (2026-11-07): pasting into the Code field works: the pasted text is added with line breaks and surrounding whitespace removed. Typed and pasted code characters are limited to printable ASCII, as the GitHub Enterprise host field already is. Paste into the GitHub Enterprise host field works the same way and clears its validation error.
  #   4. User request (2026-11-07): long text on the sign-in screens wraps to the screen width instead of being cut off: the authorize URL, the typed code, the device code and verification URL, and the error text. Every character is shown at 80 and at 120 columns. A URL starts on a row of its own, so a terminal Shift+drag over its rows takes only URL characters.
  #   5. User request (2026-11-07): Ctrl+C, the copy key of the Provider Settings fields (PROV-138), copies on every sign-in screen: on code entry the URL while the URL is focused and the typed code while the Code field is focused; on device login the user code; on the error screen the error text; on the GitHub Enterprise host field the typed host. None of these is a secret, so nothing is masked. A copy always holds the unwrapped text.
  #   6. Device login (Codex sign in with code and GitHub Copilot): c copies the user code and o opens the verification URL. While the device code is still being requested the screen says so and c and o do nothing.
  #   7. The OAuth error screen: c copies the error text.
  #   8. The browser sign-in waiting screen says what to do when no browser opens: press Esc and use the provider's sign-in-with-code row. (Showing the local login URL there needs a new backend call; it is listed as a follow-up, not done here.)
  #   9. Every copy through Action::CopyToClipboard shows a short confirmation for 2 seconds that says how many characters were copied; a failed write says it failed and why. This App-level copy notice is the shared piece TUI-130 reuses for every other copy.
  #   10. The Provider Settings status line is shown on the sign-in screens too (it carries the browser-open failure). In List mode the status wraps to at most 3 rows instead of being cut off, and Ctrl+C in List mode copies a non-empty status (the profile-form status lines are TUI-131's wrap scope).
  #   11. The key hints on each sign-in screen (body line and footer) list exactly the keys that work in the current state and focus.
  #   12. Out of scope here: PS-14 and PS-15 of the audit (unreachable refresh-models screen, List footer mismatches) are TUI-132's cleanup scope as that card says.
  #
  # EXAMPLES:
  #   1. Anthropic sign in with code shows a 384-character authorize URL; pressing c puts exactly that URL on the clipboard (OSC 52) and the screen confirms 'Copied 384 characters'
  #   2. Pressing o on the URL opens the URL in the browser; when the launcher fails the screen shows 'Could not open a browser: <reason>. Press c to copy the URL.'
  #   3. After c the focus stays on the URL; Tab focuses the Code field, typing 'co9#st' keeps the c and o, and Enter submits it
  #   4. Pasting 'abc#xyz\n' on the URL puts 'abc#xyz' in the Code field; Enter submits 'abc#xyz'
  #   5. At 80 columns and at 120 columns the whole authorize URL is on screen across the rows under 'Visit:', and a 90-character pasted code is fully visible
  #   6. Ctrl+C on the URL copies the URL; Ctrl+C in the Code field holding 'abc' copies 'abc'
  #   7. Codex device login shows user code 'ABCD-1234' and a verification URL: c copies 'ABCD-1234', o opens the verification URL, Ctrl+C copies 'ABCD-1234'
  #   8. GitHub Copilot device login before the device code arrives shows 'Requesting device code…' and c copies nothing
  #   9. A 250-character OAuth error is fully visible at 80 columns; c and Ctrl+C copy the error text
  #   10. Pasting 'ghe.example.com' into the GitHub Enterprise host field shows it and clears the validation error; Ctrl+C copies it
  #   11. The browser waiting screen says 'Browser didn't open? Press Esc and use Sign in with code.'
  #   12. A 200-character List status is fully visible over at most 3 rows at 80 columns and Ctrl+C copies it
  #   13. In the Code field the hints no longer offer c and o; on the URL they offer c: copy URL and o: open URL
  #
  # ========================================
  Background: User Story
    As a user signing in to a provider from Provider Settings (Anthropic sign in with code, Codex or GitHub Copilot device login)
    I want to copy, open, read and paste the sign-in URL, device code, my code and any error on the sign-in screens
    So that I can finish signing in without retyping a 384-character URL or a 90-character code, at any terminal width

  Scenario: c copies the whole authorize URL and the copy is confirmed
    Given Anthropic sign in with code shows a 384-character authorize URL
    When I press c
    Then the clipboard receives exactly the 384-character URL
    And the screen confirms "Sent 384 characters to the terminal clipboard (OSC 52)"
    And the copy confirmation is gone 2 seconds later

  Scenario: o opens the authorize URL and a failed open says how to copy it
    Given Anthropic sign in with code shows the authorize URL
    When I press o
    Then the URL launcher is asked to open the authorize URL
    When I press o again and the launcher fails with "no browser"
    Then the screen shows "Could not open a browser: no browser. Press c to copy the URL."

  Scenario: A code that starts with c or o keeps its first character
    Given Anthropic sign in with code shows the authorize URL
    When I press c to copy the URL
    And I press Tab to focus the Code field
    And I type "co9#st"
    Then the Code field holds "co9#st"
    When I press Enter
    Then the code "co9#st" is submitted

  Scenario: A pasted code goes into the Code field without its line break
    Given Anthropic sign in with code shows the authorize URL
    When I paste "abc#xyz" followed by a line break
    Then the Code field holds "abc#xyz"
    When I press Enter
    Then the code "abc#xyz" is submitted

  Scenario: The authorize URL and a long code wrap at 80 and at 120 columns
    Given Anthropic sign in with code shows a 384-character authorize URL
    And I pasted a 90-character code
    When the screen is drawn 80 columns wide and again 120 columns wide
    Then each time the rows under "Visit:" hold the whole URL and nothing else
    And each time the whole code is on screen

  Scenario: Ctrl+C copies the focused URL or the typed code
    Given Anthropic sign in with code shows the authorize URL
    When I press Ctrl+C
    Then the clipboard receives the authorize URL
    When I press Tab, type "abc" and press Ctrl+C
    Then the clipboard receives "abc"

  Scenario: Device login copies the user code and opens the verification URL
    Given Codex device login shows the user code "ABCD-1234" and a verification URL
    When I press c
    Then the clipboard receives "ABCD-1234"
    When I press Ctrl+C
    Then the clipboard receives "ABCD-1234" again
    When I press o
    Then the URL launcher is asked to open the verification URL

  Scenario: Device login waiting for its code says so and copies nothing
    Given GitHub Copilot device login is waiting for its device code
    When the screen is drawn
    Then it shows "Requesting device code…"
    When I press c
    Then nothing is copied

  Scenario: A long OAuth error is shown whole and can be copied
    Given the sign-in failed with a 250-character error
    When the screen is drawn 80 columns wide
    Then the whole error is on screen
    When I press c
    Then the clipboard receives the error text
    When I press Ctrl+C
    Then the clipboard receives the error text again

  Scenario: The GitHub Enterprise host field takes a paste and copies it
    Given the GitHub Enterprise host field shows a validation error
    When I paste "ghe.example.com"
    Then the field holds "ghe.example.com" and the validation error is gone
    When I press Ctrl+C
    Then the clipboard receives "ghe.example.com"

  Scenario: The browser waiting screen says what to do when no browser opens
    Given Anthropic browser sign-in is waiting for authorization
    When the screen is drawn
    Then it shows "Browser didn't open? Press Esc and use Sign in with code."

  Scenario: A long List status wraps and Ctrl+C copies it
    Given Provider Settings shows a 200-character status in List mode
    When the screen is drawn 80 columns wide
    Then the whole status is on screen over at most 3 rows
    When I press Ctrl+C
    Then the clipboard receives the status

  Scenario: The hints follow the focus on the code-entry screen
    Given Anthropic sign in with code shows the authorize URL
    When the screen is drawn
    Then the hints offer "c: copy URL" and "o: open URL"
    When I press Tab to focus the Code field
    Then the hints no longer offer c or o
