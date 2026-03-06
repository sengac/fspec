# WebMCP Dynamic Tool Registration — Research & Resolution

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Architecture Overview](#architecture-overview)
3. [W3C WebMCP Specification Analysis](#w3c-webmcp-specification-analysis)
4. [Chromium Implementation Deep Dive](#chromium-implementation-deep-dive)
5. [Current fspec Extension Architecture](#current-fspec-extension-architecture)
6. [Root Cause Analysis](#root-cause-analysis)
7. [The webmcp.dev Polyfill Problem](#the-webmcpdev-polyfill-problem)
8. [Resolution Options](#resolution-options)
9. [Recommended Solution](#recommended-solution)
10. [Implementation Plan](#implementation-plan)
11. [References](#references)

> **Research date:** 2026-03-06
> **Chromium source indexed at:** `main` branch (current)

---

## Problem Statement

When a user dynamically registers a WebMCP tool on a website (e.g., clicking
"Register Weather Tool" on webmcp.dev), the tool does **not** appear in the
MCP tool list exposed to the connected AI agent. Reconnecting the MCP client
still shows only the 11 native browser tools. The dynamic WebMCP tools are
never surfaced.

---

## Architecture Overview

The fspec WebMCP extension bridges website-registered tools to AI agents via
a multi-hop pipeline:

```
┌─────────────────────────────────────────────────────────────────┐
│ Web Page (MAIN world)                                           │
│                                                                 │
│  navigator.modelContext.registerTool()                          │
│       │                                                         │
│       ▼                                                         │
│  Monkey-patched interceptor (webmcp-discovery.ts)               │
│       │                                                         │
│       ▼ window.postMessage('FSPEC_WEBMCP_TOOL_REGISTERED')      │
└───────┬─────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│ Content Script (ISOLATED world) — relay.ts                      │
│                                                                 │
│  window.addEventListener('message') → chrome.runtime.sendMessage│
└───────┬─────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│ Service Worker — message-router.ts                              │
│                                                                 │
│  handleContentScriptMessage() → toolRegistry.register()         │
│       │                                                         │
│       ▼ notifyToolsChanged() → port.postMessage(TOOLS_CHANGED)  │
└───────┬─────────────────────────────────────────────────────────┘
        │ Native Messaging (stdin/stdout, 4-byte framing)
        ▼
┌─────────────────────────────────────────────────────────────────┐
│ Native Host — mcp-server.mjs                                    │
│                                                                 │
│  handleNativeMessage(TOOLS_CHANGED) → session.tools = [...]     │
│       │                                                         │
│       ▼ SSE: notifications/tools/list_changed                   │
│                                                                 │
│  tools/list handler returns [...NATIVE_TOOLS, ...session.tools] │
└───────┬─────────────────────────────────────────────────────────┘
        │ HTTP Streamable MCP (port 19876)
        ▼
┌─────────────────────────────────────────────────────────────────┐
│ MCP Client (Claude Code, etc.)                                  │
│                                                                 │
│  ConnectMCP → tools/list → sees browser tools + WebMCP tools    │
└─────────────────────────────────────────────────────────────────┘
```

---

## W3C WebMCP Specification Analysis

Source: [W3C Community Group Draft](https://webmachinelearning.github.io/webmcp/)

### Public API Surface

The W3C spec defines `ModelContext` with **only two methods**:

```webidl
[Exposed=Window, SecureContext]
interface ModelContext {
  undefined registerTool(ModelContextTool tool);
  undefined unregisterTool(DOMString name);
};

partial interface Navigator {
  [SecureContext, SameObject] readonly attribute ModelContext modelContext;
};
```

### What the spec does NOT provide

- ❌ No `EventTarget` inheritance — `ModelContext` is not an event emitter
- ❌ No `addEventListener()` / `ontoolchange` event handler
- ❌ No `MutationObserver`-style observation API
- ❌ No callback for tool list changes
- ❌ No `listTools()` for enumerating registered tools

The spec treats `registerTool()` and `unregisterTool()` as fire-and-forget
mutations to an internal `tool map`. The browser mediates between the page
and consuming agents, but the spec defines **no extension-facing
notification channel**.

### Additional methods (polyfill-only, non-standard)

The [`@mcp-b/global`](https://www.npmjs.com/package/@mcp-b/global) polyfill
references two additional methods not in the W3C spec:

- `provideContext(options)` — replaces all "base" tools atomically
- `clearContext()` — removes all tools

These are also fire-and-forget with no notification mechanism.

---

## Chromium Implementation Deep Dive

The Chromium implementation lives in `third_party/blink/renderer/core/script_tools/`.

### Source files

| File | Description |
|------|-------------|
| [`model_context.idl`][idl] | WebIDL — defines the public JS API (2 methods) |
| [`model_context.h`][h] | C++ header — includes internal `SetToolsChangedCallback` |
| [`model_context.cc`][cc] | C++ implementation — calls `OnToolsChanged()` on every mutation |
| [`model_context_testing.idl`][test-idl] | WebIDL for the **testing** interface (behind `WebMCPTesting` flag) |
| [`model_context_testing.cc`][test-cc] | Testing impl — fires `toolchange` event |
| [`script_tools.mojom`][mojom] | Mojo IPC — renderer↔browser boundary (only `PauseExecution`) |

[idl]: https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context.idl
[h]: https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context.h
[cc]: https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context.cc
[test-idl]: https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context_testing.idl
[test-cc]: https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context_testing.cc
[mojom]: https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/public/mojom/content_extraction/script_tools.mojom

### Internal `OnToolsChanged()` callback mechanism

The `ModelContext` C++ class has an internal notification system that is
**not** exposed to web pages:

```cpp
// model_context.h, line 78-80
void SetToolsChangedCallback(std::optional<base::RepeatingClosure> cb) {
    tools_changed_closure_ = std::move(cb);
}
```

`OnToolsChanged()` is called on every `registerTool()`, `unregisterTool()`,
and `RegisterDeclarativeTool()` call:

```cpp
// model_context.cc
void ModelContext::registerTool(...) {
    // ... validation, build tool_data ...
    tool_map_.insert(tool->name(), tool_data);
    OnToolsChanged();  // <-- fires here
}

void ModelContext::unregisterTool(const String& name, ...) {
    // ... lookup ...
    tool_map_.erase(it);
    OnToolsChanged();  // <-- fires here
}

void ModelContext::OnToolsChanged() {
    if (tools_changed_closure_) {
        tools_changed_closure_->Run();
    }
}
```

### The `ModelContextTesting` interface (behind `WebMCPTesting` flag)

Chrome exposes a **testing-only** interface that hooks into the internal
callback and provides everything an extension would need:

```webidl
// model_context_testing.idl
// Gated by: RuntimeEnabled=WebMCPTesting
interface ModelContextTesting : EventTarget {
    sequence<RegisteredTool> listTools();
    Promise<DOMString?> executeTool(DOMString tool_name, DOMString input_arguments, ...);
    undefined registerToolsChangedCallback(ToolsChangedCallback callback);
    attribute EventHandler ontoolchange;   // ← DOM event!
};
```

The implementation registers itself as the `tools_changed_closure_`:

```cpp
// model_context_testing.cc
ModelContextTesting::ModelContextTesting(ModelContext& model_context)
    : model_context_(model_context) {
    model_context_->SetToolsChangedCallback(blink::BindRepeating(
        &ModelContextTesting::OnToolsChanged, WrapWeakPersistent(this)));
}

void ModelContextTesting::OnToolsChanged() {
    // Fires a real DOM event
    DispatchEvent(*Event::Create(event_type_names::kToolchange));
    // Also invokes the JS callback if registered
    if (tools_changed_callback_) {
        tools_changed_callback_->Invoke(nullptr);
    }
}
```

### Mojo IPC boundary

The [`script_tools.mojom`][mojom] is minimal — it only defines
`PauseExecution()`. There is **no** Mojo message for "tools changed"
crossing the renderer→browser process boundary. This means:

- The browser process itself does not receive tool-change notifications
- Extensions cannot use any Chrome extension API to observe tool changes
- The only path is in-renderer interception (main world script injection)

### Key finding

Chrome has the exact notification mechanism needed (`ontoolchange` event +
`listTools()` + `executeTool()`), but it is:

1. Behind the `WebMCPTesting` flag (separate from the main `WebMCP` flag)
2. Available only via `ModelContextTesting`, not `ModelContext`
3. Not exposed through any Chrome extension API

---

## Current fspec Extension Architecture

### Discovery strategy: monkey-patching

The extension injects a MAIN-world script (`webmcp-discovery.ts`) via
`chrome.scripting.executeScript({ world: 'MAIN' })`. This script:

1. Checks if `navigator.modelContext` already exists
2. If yes, wraps `registerTool()` and `unregisterTool()` with interceptors
3. If no, uses `Object.defineProperty()` to trap future assignment
4. On interception, fires `window.postMessage('FSPEC_WEBMCP_TOOL_REGISTERED')`
5. Stores the `execute` callback for later invocation

### Injection trigger

The `webmcp-injector.ts` injects the script on `chrome.tabs.onUpdated` when
`changeInfo.status === 'complete'`. It tracks injected tabs to avoid
double-injection and clears the tracking on navigation (re-injects).

### Content script relay

The content script (`relay.ts`) runs in the ISOLATED world and bridges:
- **Main → SW:** `FSPEC_WEBMCP_*` messages forwarded via `chrome.runtime.sendMessage()`
- **SW → Main:** `FSPEC_INVOKE_TOOL` messages forwarded via `window.postMessage()`

### Service worker routing

The `message-router.ts` in the service worker:
- Receives `FSPEC_WEBMCP_TOOL_REGISTERED` → registers in `toolRegistry`
- Calls `notifyToolsChanged()` → sends `TOOLS_CHANGED` to native host
- The native host updates `session.tools` and sends SSE notification

---

## Root Cause Analysis

There are **two distinct failure modes** preventing dynamic tool discovery.

### Failure Mode 1: Polyfill bypasses `navigator.modelContext`

The webmcp.dev site uses the `WebMCP` class from
[jasonjmcghee/WebMCP](https://github.com/jasonjmcghee/WebMCP):

```html
<script src="src/webmcp.js"></script>
<script>
  window.webMCP = new WebMCP();
  const mcp = window.webMCP;
  mcp.registerTool('calculator', ...);   // NOT navigator.modelContext
</script>
```

The `WebMCP` class maintains its **own internal tool registry** and
communicates with connected MCP clients via its own WebSocket-based
protocol. It does **not** use `navigator.modelContext.registerTool()` at
all. Therefore:

- The fspec monkey-patch on `navigator.modelContext` **never fires**
- Tools registered via `mcp.registerTool()` are invisible to the extension
- This is the primary failure on webmcp.dev

### Failure Mode 2: Race condition on native `navigator.modelContext`

Even for sites using the native Chrome API (with the `WebMCP` flag enabled),
there's a timing issue:

1. Content script injection is `"run_at": "document_idle"`
2. MAIN-world injection happens on `tabs.onUpdated` status `'complete'`
3. If a page calls `navigator.modelContext.registerTool()` during initial
   script execution (before `'complete'` fires), the monkey-patch isn't in
   place yet and misses those registrations

The `Object.defineProperty` trap on `navigator.modelContext` would catch
the *assignment* of the `modelContext` object, but if Chrome provides it
natively (not via polyfill assignment), the trap never fires because
`navigator.modelContext` already exists as a native IDL attribute.

### Failure Mode 3: `inputSchema` format mismatch

The `ToolRegistryEntry` in the MCP server expects tools in MCP format with
an `inputSchema` object. But the `TOOLS_CHANGED` message sends the raw
registry entries from the extension. The `session.tools` array is spread
directly into the `tools/list` response alongside `NATIVE_TOOLS`. If the
tool entry shape doesn't exactly match MCP's expected format (with `name`,
`description`, `inputSchema` at the top level), the MCP client may silently
ignore malformed tools.

---

## The webmcp.dev Polyfill Problem

The webmcp.dev site represents a **class of WebMCP implementations** that
do not use the W3C `navigator.modelContext` API. Instead, they use their
own library-specific registration API (`new WebMCP()` + `mcp.registerTool()`).

This creates a fundamental mismatch:

| Approach | How tools are registered | fspec can detect? |
|----------|------------------------|-------------------|
| Native Chrome API (`WebMCP` flag) | `navigator.modelContext.registerTool()` | ✅ Via monkey-patch |
| `@mcp-b/global` polyfill | Sets `navigator.modelContext`, then `registerTool()` | ✅ Via `Object.defineProperty` trap |
| `WebMCP` library (webmcp.dev) | `new WebMCP().registerTool()` — own internal registry | ❌ Not detected |
| Declarative `<form>` tools | HTML attributes `toolname`, `tooldescription` | ❌ Not detected |

The webmcp.dev `WebMCP` class has its own WebSocket-based MCP transport
and widget UI. It is a completely separate tool ecosystem from the native
browser API. The fspec extension's monkey-patching strategy cannot see
these tools because they never touch `navigator.modelContext`.

---

## Resolution Options

### Option A: Intercept the `WebMCP` library class directly

**How:** In `webmcp-discovery.ts`, also detect and monkey-patch
`window.WebMCP.prototype.registerTool` or any `WebMCP` instance.

**Pros:**
- Works with webmcp.dev and similar sites using this specific library

**Cons:**
- Brittle — tied to one library's internal API
- Would need updating for every WebMCP library version change
- Doesn't generalise to other polyfill libraries
- Unclear how many variants exist in the wild

**Verdict:** ❌ Not recommended — too fragile and narrow.

### Option B: DOM MutationObserver for declarative tools

**How:** Watch for `<form>` elements with `toolname` attributes being
added to the DOM. This covers the declarative WebMCP path.

**Pros:**
- Catches declarative tools that neither JS API would detect
- Uses standard, stable DOM APIs

**Cons:**
- Only covers declarative `<form>`-based tools, not imperative JS tools
- Complementary to other approaches, not a standalone solution

**Verdict:** ⚠️ Useful as supplement, but doesn't solve the main problem.

### Option C: Periodic polling via `chrome.scripting.executeScript`

**How:** Periodically inject a MAIN-world script that reads
`navigator.modelContext`'s internal tool map and reports any changes.

**Pros:**
- Catches tools registered before the monkey-patch was in place
- Could detect native Chrome API tools even after the race window

**Cons:**
- Polling is wasteful and adds latency
- Cannot read internal tool map — `ModelContext` has no `listTools()` in
  the public API (only in `ModelContextTesting`)
- Still can't see `WebMCP` library tools

**Verdict:** ❌ Not viable — no public `listTools()` method to poll.

### Option D: Use `ModelContextTesting` API when available

**How:** If the `WebMCPTesting` flag is enabled, access
`ModelContextTesting.listTools()` and `ontoolchange` event from the
injected MAIN-world script.

**Pros:**
- Uses Chrome's own notification mechanism — clean, no monkey-patching
- `listTools()` catches tools registered before injection
- `ontoolchange` provides real-time notifications

**Cons:**
- Requires the user to enable a separate Chrome flag (`WebMCPTesting`)
- Testing API, not guaranteed to be stable
- Only works with native Chrome `navigator.modelContext`, not polyfills

**Verdict:** ⚠️ Good enhancement but limited audience.

### Option E: Universal `registerTool` interception (recommended)

**How:** In the MAIN-world discovery script, intercept `registerTool` at
**multiple layers**:

1. **`navigator.modelContext`** — current approach (kept)
2. **`WebMCP.prototype`** — if `window.WebMCP` exists, wrap its
   `registerTool` method
3. **Generic trap** — use `Object.defineProperty` on `window` to watch for
   common polyfill patterns setting `window.webMCP`, `window.mcp`, etc.
4. **Periodic snapshot** — after initial page load, do a one-time check
   for any `WebMCP` instances on `window` and retroactively discover tools

**Pros:**
- Covers both native and polyfill paths
- Single injection point, multiple detection strategies
- Can discover tools registered before injection via snapshot

**Cons:**
- More complex discovery script
- Still can't cover every possible proprietary tool registry
- Needs maintenance as new polyfill patterns emerge

**Verdict:** ✅ Best pragmatic approach.

### Option F: Standardise an extension-facing API (long-term)

**How:** Propose a Chrome extension API (like `chrome.modelContext.onToolsChanged`)
that exposes tool change notifications to extensions directly from the
browser process.

**Pros:**
- Clean, officially supported, no hacks needed
- Would work for all tool registration paths (native, declarative, polyfill)

**Cons:**
- Requires Chrome team buy-in and implementation
- Timeline: months to years
- The Mojo IPC boundary currently has no tool-change messages

**Verdict:** 📋 File as a feature request; not actionable now.

---

## Recommended Solution

A **layered discovery strategy** in `webmcp-discovery.ts` that combines
multiple detection methods with a one-time retroactive scan.

### Layer 1: `navigator.modelContext` interception (existing)

Keep the current monkey-patching of `navigator.modelContext.registerTool()`
and `unregisterTool()`. This covers:

- Native Chrome API (when `WebMCP` flag is enabled)
- `@mcp-b/global` polyfill (which sets `navigator.modelContext`)
- Any future polyfill that follows the W3C spec

### Layer 2: `WebMCP` class interception (new)

Detect and intercept the `WebMCP` library used by webmcp.dev and similar
sites:

```javascript
// Watch for WebMCP constructor instances
const OriginalWebMCP = window.WebMCP;
if (OriginalWebMCP) {
  wrapWebMCPClass(OriginalWebMCP);
}
// Also trap future assignment
Object.defineProperty(window, 'WebMCP', {
  configurable: true,
  get() { return OriginalWebMCP; },
  set(NewClass) {
    OriginalWebMCP = NewClass;
    wrapWebMCPClass(NewClass);
  }
});

function wrapWebMCPClass(WebMCPClass) {
  const origRegister = WebMCPClass.prototype.registerTool;
  WebMCPClass.prototype.registerTool = function(name, desc, schema, fn) {
    // Notify extension
    window.postMessage({
      type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
      tool: { name, description: desc, inputSchema: schema },
      origin: pageOrigin,
    }, '*');
    // Store execute fn for invocation
    registeredTools.set(name, fn);
    // Call original
    return origRegister.call(this, name, desc, schema, fn);
  };
}
```

### Layer 3: Post-load snapshot (new)

After `document_idle` / `DOMContentLoaded`, do a one-time scan for any
`WebMCP` instances on well-known globals (`window.webMCP`, `window.mcp`)
and extract already-registered tools:

```javascript
// After a short delay to let page scripts finish
setTimeout(() => {
  // Check for WebMCP library instances
  for (const key of ['webMCP', 'mcp', 'webmcp']) {
    const instance = window[key];
    if (instance && typeof instance.getTools === 'function') {
      for (const tool of instance.getTools()) {
        if (!registeredTools.has(tool.name)) {
          window.postMessage({
            type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
            tool: { name: tool.name, description: tool.description,
                    inputSchema: tool.inputSchema },
            origin: pageOrigin,
          }, '*');
        }
      }
    }
  }
}, 500);
```

### Layer 4: `ModelContextTesting` (new, opportunistic)

If Chrome's `WebMCPTesting` flag is enabled, use the testing API for clean
notifications instead of monkey-patching:

```javascript
// Feature-detect the testing interface
if (navigator.modelContext && navigator.modelContext.testing) {
  const testing = navigator.modelContext.testing;
  testing.ontoolchange = () => {
    const tools = testing.listTools();
    // Diff against known tools and notify extension of changes
  };
}
```

---

## Implementation Plan

### Phase 1: Fix the immediate WebMCP library problem

**Files to modify:**
- `extension/src/content/webmcp-discovery.ts`

**Changes:**
1. Add `WebMCP.prototype.registerTool` interception
2. Add `Object.defineProperty` trap for `window.WebMCP`
3. Add post-load snapshot for `window.webMCP` instances
4. Ensure `execute` callbacks are captured for invocation

**Estimated effort:** 2–3 hours

### Phase 2: Fix the injection timing race

**Files to modify:**
- `extension/manifest.json` — change `"run_at": "document_idle"` to
  `"document_start"` for earlier content script injection
- `extension/src/background/webmcp-injector.ts` — also inject on
  `document_start` via `chrome.scripting.executeScript` with
  `injectImmediately: true`

**Changes:**
1. Inject the MAIN-world script as early as possible
2. Use both `Object.defineProperty` traps (for native) and prototype
   wrapping (for polyfill) to catch all registration paths
3. Handle the case where `navigator.modelContext` is a native readonly
   attribute (can't be trapped via `defineProperty`)

**Estimated effort:** 1–2 hours

### Phase 3: Add `ModelContextTesting` support (opportunistic)

**Files to modify:**
- `extension/src/content/webmcp-discovery.ts`

**Changes:**
1. Feature-detect `ModelContextTesting` availability
2. If available, use `ontoolchange` + `listTools()` instead of monkey-patch
3. If available, use `executeTool()` instead of stored callbacks
4. Fall back to monkey-patching when testing API is unavailable

**Estimated effort:** 1–2 hours

### Phase 4: End-to-end validation

1. Test with webmcp.dev (WebMCP library — polyfill path)
2. Test with Chrome native `navigator.modelContext` (WebMCP flag)
3. Test with `@mcp-b/global` polyfill
4. Test tool registration before and after page load
5. Test dynamic registration (button click)
6. Test tool unregistration
7. Test tool invocation through the MCP client
8. Verify `notifications/tools/list_changed` SSE is sent to connected agents

---

## References

### W3C Specification
- [WebMCP W3C Community Group Draft](https://webmachinelearning.github.io/webmcp/)
- [WebMCP GitHub — webmachinelearning/webmcp](https://github.com/webmachinelearning/webmcp)

### Chromium Source
- [`model_context.idl`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context.idl) — Public WebIDL (2 methods)
- [`model_context.h`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context.h) — C++ header with `SetToolsChangedCallback`
- [`model_context.cc`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context.cc) — Implementation calling `OnToolsChanged()`
- [`model_context_testing.idl`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context_testing.idl) — Testing interface with `ontoolchange` event
- [`model_context_testing.cc`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/script_tools/model_context_testing.cc) — Testing implementation
- [`script_tools.mojom`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/public/mojom/content_extraction/script_tools.mojom) — Mojo IPC (renderer↔browser)

### Polyfill / Library
- [`@mcp-b/global` npm package](https://www.npmjs.com/package/@mcp-b/global) — W3C polyfill
- [`@mcp-b/global` docs](https://docs.mcp-b.ai/packages/global) — API reference
- [jasonjmcghee/WebMCP](https://github.com/jasonjmcghee/WebMCP) — Library used by webmcp.dev

### Articles
- [Chrome's WebMCP Early Preview (dev.to)](https://dev.to/axrisi/chromes-webmcp-early-preview-the-end-of-ai-agents-clicking-buttons-b6e) — mentions `provideContext()`, `clearContext()`
- [WebMCP updates and next steps (Patrick Brosset)](https://patrickbrosset.com/articles/2026-02-23-webmcp-updates-clarifications-and-next-steps/) — API naming transition

### fspec Extension Source
- [`extension/src/content/webmcp-discovery.ts`](extension/src/content/webmcp-discovery.ts) — Main-world monkey-patch
- [`extension/src/content/relay.ts`](extension/src/content/relay.ts) — Content script relay
- [`extension/src/background/webmcp-injector.ts`](extension/src/background/webmcp-injector.ts) — Tab injection trigger
- [`extension/src/background/message-router.ts`](extension/src/background/message-router.ts) — Service worker routing
- [`extension/src/background/tool-registry.ts`](extension/src/background/tool-registry.ts) — Tool registry
- [`extension/src/background/service-worker.ts`](extension/src/background/service-worker.ts) — Service worker entry
- [`extension/host/lib/mcp-server.mjs`](extension/host/lib/mcp-server.mjs) — Native host MCP server
