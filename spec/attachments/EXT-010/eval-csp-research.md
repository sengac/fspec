# browser_execute_script Returns Null — CSP Blocks eval() in Extension Isolated World

## Problem

`browser_execute_script` always returns `"null"` regardless of what code is passed. Every other
browser tool (e.g. `browser_get_page_content`, `browser_click_element`) works correctly.

### Reproduction

```
browser_execute_script({ code: "document.title" })        → "null"
browser_execute_script({ code: "1 + 1" })                 → "null"
browser_execute_script({ code: "JSON.stringify({a:1})" })  → "null"
browser_get_page_content()                                 → works fine
```

Tested on google.com, httpbin.org, webmcp.dev — all return `"null"`.

## Root Cause

The problem is in `extension/src/background/browser-tools.ts` lines 211–217:

```typescript
handlers.set('browser_execute_script', async args => {
  const results = await scripting.executeScript({
    target: { tabId },
    args: [code],
    func: (codeStr: string) => {
      return eval(codeStr);    // ← CSP blocks this
    },
  });
});
```

### Why eval() fails

1. `chrome.scripting.executeScript({ func })` injects the serialized function into the
   extension's **ISOLATED world** (the default when no `world` is specified).

2. Chrome Manifest V3 enforces a strict Content Security Policy on extension contexts.
   The extension CSP **does not allow `unsafe-eval`** — this is a hard MV3 restriction
   that cannot be overridden via `manifest.json`.

3. When `eval()` is called inside the injected function, Chromium raises an `EvalError`:
   `"Refused to evaluate a string as JavaScript because 'unsafe-eval' is not an allowed
   source of script in the following Content Security Policy directive"`.

4. Chromium catches this error internally and returns `null` as the injection result.
   The error is **silently swallowed** — no exception propagates to the `executeScript`
   caller.

### Why other tools work

`browser_get_page_content`, `browser_click_element`, `browser_fill_form` all use
`chrome.scripting.executeScript` with a `func` that contains **inline code** (direct DOM
access). They never call `eval()`, so CSP doesn't block them. The function body itself
is serialized and injected as trusted extension code — only `eval()` / `new Function()`
string-to-code conversion is blocked.

### Specifying `world: 'MAIN'` doesn't help either

Injecting into the MAIN world would bypass the extension CSP, but the **page's own CSP**
would then block `eval()`. Most modern websites (Google, GitHub, Twitter, etc.) have strict
CSP headers that forbid `unsafe-eval`.

## Solutions Investigated

### Option 1: `chrome.userScripts` API (RECOMMENDED)

**Available since:** Chrome 120 (`configureWorld`), Chrome 135 (`execute`)
**Current Chrome:** 145 ✅

The `chrome.userScripts` API provides a `USER_SCRIPT` execution world that is:
- **Exempt from the page's CSP** (per Chrome documentation)
- **Configurable with a custom CSP** via `configureWorld()`

```typescript
// On extension startup — configure the USER_SCRIPT world to allow eval:
chrome.userScripts.configureWorld({
  csp: "script-src 'self' 'unsafe-eval' 'unsafe-inline'"
});

// To execute arbitrary code:
const results = await chrome.userScripts.execute({
  target: { tabId },
  world: 'USER_SCRIPT',
  js: [{ code: userProvidedCode }],
});
```

**Requirements:**
- Add `"userScripts"` permission to `manifest.json`
- User must enable "Allow User Scripts" toggle in extension settings (Chrome 138+)
  or have Developer Mode enabled (Chrome < 138)

**Tradeoffs:**
- ✅ Official Chrome-blessed API for running arbitrary code
- ✅ This is what Tampermonkey/Violentmonkey plan to use for MV3
- ✅ Returns results properly (has `InjectionResult` with `.result`)
- ⚠️ Requires user to enable a toggle (acceptable for developer tool)
- ⚠️ `chrome.userScripts.execute()` is Chrome 135+ only

**Source:** [Chrome userScripts API docs](https://developer.chrome.com/docs/extensions/reference/api/userScripts),
[Violentmonkey discussion #2135](https://github.com/violentmonkey/violentmonkey/discussions/2135)

### Option 2: `declarativeNetRequest` — Strip CSP Headers

**How it works:** Use `chrome.declarativeNetRequest.updateSessionRules()` to remove the
`Content-Security-Policy` response header from pages before they load.

```typescript
chrome.declarativeNetRequest.updateSessionRules({
  addRules: [{
    id: 1,
    priority: 1,
    action: {
      type: 'modifyHeaders',
      responseHeaders: [{
        header: 'content-security-policy',
        operation: 'remove',
      }],
    },
    condition: {
      tabIds: [tabId],
      resourceTypes: ["main_frame", "sub_frame"]
    },
  }],
  removeRuleIds: [1],
});
```

With CSP headers stripped, `eval()` in the MAIN world works. This is how
[chrome-csp-disable](https://github.com/PhilGrayson/chrome-csp-disable) works.

**Tradeoffs:**
- ✅ No user toggle required beyond standard permissions
- ✅ eval() works in MAIN world after page reload
- ❌ Weakens security for every page the extension touches
- ❌ Requires page reload after setting the rule (CSP is evaluated at load time)
- ❌ `content-security-policy-report-only` header must also be stripped
- ❌ Some sites use `<meta http-equiv="Content-Security-Policy">` in HTML — header
  stripping doesn't remove those

**Source:** [PhilGrayson/chrome-csp-disable](https://github.com/PhilGrayson/chrome-csp-disable)

### Option 3: `new Function()` Instead of `eval()`

Replace `eval(codeStr)` with `new Function('return (' + codeStr + ')')()`.

**Result:** Same problem — `new Function()` is also blocked by CSP `script-src` without
`unsafe-eval`. This is not a workaround.

### Option 4: `--disable-web-security` Chrome Launch Flag

Launch Chrome with:
```bash
chrome --disable-web-security --user-data-dir=/tmp/chrome-dev
```

**Tradeoffs:**
- ✅ Disables ALL security (CSP, CORS, etc.)
- ❌ Not practical for daily use
- ❌ Requires restarting Chrome
- ❌ Can't be set from the extension itself

### Option 5: Avoid eval() Entirely — Use Function-Based Execution

Instead of accepting arbitrary code strings, restructure `browser_execute_script` to
accept a function name and arguments, then dispatch to pre-defined functions.

**Tradeoffs:**
- ✅ No CSP issues at all
- ❌ Loses the ability to run arbitrary JavaScript
- ❌ Would need a huge library of pre-built functions
- ❌ Defeats the purpose of `execute_script`

## How Violentmonkey Handles This

From reading [violentmonkey/src/injected/content/inject.js](https://github.com/violentmonkey/violentmonkey/blob/master/src/injected/content/inject.js):

1. **They do NOT use `eval()`** — userscripts are injected as `<script>` elements with
   `textContent` (Chrome) or `Blob URL` + `src` attribute (Firefox 58+).

2. **CSP detection:** They check for `<meta http-equiv="content-security-policy">` in the DOM
   and test whether script injection works. Function: `didPageLoseInjectability()`.

3. **Fallback strategy:** When CSP blocks `<script>` injection in PAGE realm, they fall
   back to CONTENT realm (isolated world). This loses page context access but maintains
   functionality.

4. **Future plan:** Their MV3 migration will use `chrome.userScripts` API with
   `configureWorld({ csp: "script-src 'unsafe-eval' 'unsafe-inline'" })`.

5. **Blob URL trick** (Violentmonkey blog post): Create a Blob with the script code and
   set it as `<script src>`. This bypasses `script-src` inline restrictions but NOT
   `unsafe-eval` restrictions. Only useful for script-src, not for eval().

Source: [Inject scripts with Blob URLs](https://violentmonkey.github.io/posts/inject-scripts-with-blob-urls/)

## How Tampermonkey Handles This

Tampermonkey is closed-source, but from [issue #2270](https://github.com/Tampermonkey/tampermonkey/issues/2270)
and community analysis:

1. In MV2: Uses `chrome.webRequest.onHeadersReceived` to strip CSP headers before they
   reach the page.
2. In MV3: Uses `declarativeNetRequest` to strip CSP headers AND is migrating to
   `chrome.userScripts` API.

## Recommendation

**Use Option 1 (`chrome.userScripts` API)** as the primary approach:

1. Add `"userScripts"` permission to `manifest.json`
2. Call `chrome.userScripts.configureWorld()` on service worker startup to set a permissive
   CSP for the USER_SCRIPT world
3. Replace the `browser_execute_script` handler to use `chrome.userScripts.execute()` with
   the USER_SCRIPT world instead of `chrome.scripting.executeScript` + `eval()`
4. Fall back to `chrome.scripting.executeScript` with `func`-based approach if
   `userScripts` is unavailable (user hasn't enabled the toggle)
5. Document the "Allow User Scripts" toggle requirement in `webmcp-skill.md`

This is the same path that the major userscript managers (Violentmonkey, Tampermonkey) are
taking for MV3, and it's the officially supported Chrome API for running arbitrary code.
