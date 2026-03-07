# Perplexity AI Search — Extension Skill

Search the web via Perplexity AI directly from your AI session using the fspec Browser Agent Chrome Extension.

## Action Required

When this skill is referenced, connect to the fspec Browser Agent Chrome Extension (if not already connected):

```
ConnectMCP(transport: "http", url: "http://localhost:19876/mcp")
```

Then navigate to Perplexity:

```
browser_navigate({ url: "https://www.perplexity.ai" })
```

## How to Use This Skill

Reference this file with `@` in your prompt:

```
@extension/perplexity-skill.md search for "what is model context protocol"
```

Requires: **fspec Browser Agent Chrome Extension** installed, native host registered, and a logged-in Perplexity session in Chrome.

---

## Prerequisites

- Everything from `extension-skill.md` (extension installed, native host registered)
- A Perplexity account **already signed in** in Chrome (free tier works; answers may be shorter)

---

## Core Workflow

The efficient 3-step loop: **Submit → Extract → (optionally) New Thread**.

### Step 1: Submit a Query

Perplexity's search box is a `contenteditable` div (`#ask-input`), not a standard `<input>`. Normal form fill + Enter does not work. Instead, call React's internal `onChange` and `onSubmit` handlers through the fiber tree.

Use this single `browser_execute_script` call to submit any query:

```javascript
browser_execute_script({ code: `
(() => {
  const QUERY = 'YOUR SEARCH QUERY HERE';
  const s = document.createElement('script');
  s.textContent = \`
    (() => {
      const input = document.querySelector('#ask-input');
      if (!input) return;
      const fiberKey = Object.keys(input).find(k => k.startsWith('__reactFiber'));
      if (!fiberKey) return;
      let fiber = input[fiberKey];
      let target = fiber;
      while (target) {
        if (target.memoizedProps?.onSubmit && target.memoizedProps?.onChange) break;
        target = target.return;
      }
      if (!target) return;
      const { onChange, onSubmit } = target.memoizedProps;
      onChange('\${QUERY}', null);
      setTimeout(() => onSubmit({ query: '\${QUERY}', json: null }), 50);
    })();
  \`;
  document.head.appendChild(s);
  s.remove();
  return 'submitted';
})();
` })
```

**Why MAIN world?** React fiber properties (`__reactFiber$...`) are only visible in the page's MAIN JavaScript world. The `<script>` tag injection bridges from USER_SCRIPT world into MAIN world.

After submission, Perplexity navigates to a `/search/...` URL. Wait for the page to load — check with `browser_list_tabs` (URL will change from `perplexity.ai/` to `perplexity.ai/search/...`).

### Step 2: Extract the Answer

Perplexity renders answers inside `.prose` blocks. Extract clean text by stripping inline source annotations:

```javascript
browser_execute_script({ code: `
(() => {
  const proseBlocks = document.querySelectorAll('.prose');
  if (proseBlocks.length === 0) return JSON.stringify({ status: 'loading' });

  const lastProse = proseBlocks[proseBlocks.length - 1];
  let text = lastProse.innerText;

  // Strip source annotations (domain names, +N counters, zero-width spaces)
  text = text.split('\\n').filter(line => {
    const t = line.trim();
    if (!t || t === '\\u200b') return false;
    if (/^\\+\\d+$/.test(t)) return false;
    if (/^[a-z][a-z0-9.-]*$/.test(t) && t.length < 30) return false;
    return true;
  }).join('\\n').replace(/\\n{3,}/g, '\\n\\n').trim();

  const h1s = document.querySelectorAll('main h1');
  const question = h1s.length > 0 ? h1s[h1s.length - 1].textContent.trim() : '';

  const main = document.querySelector('main');
  const followUps = Array.from(main?.querySelectorAll('button') || [])
    .map(b => b.textContent.trim())
    .filter(t => t.length > 20 && t.length < 100
      && !t.includes('sources') && !t.includes('Upgrade') && !t.includes('Free preview'));

  return JSON.stringify({ question, answer: text, followUps }, null, 2);
})();
` })
```

Returns a JSON object:

```json
{
  "question": "what is model context protocol",
  "answer": "The Model Context Protocol (MCP) is an open standard...",
  "followUps": [
    "How do I implement MCP in my AI application",
    "What is an MCP server and how to build one"
  ]
}
```

If the result says `{ "status": "loading" }`, the answer hasn't rendered yet — wait a moment and retry.

### Step 3: Start a New Thread (for the next query)

Each Perplexity search creates a thread. To start a fresh search (not a follow-up), click "New Thread":

```
browser_scan_page({ selector: "button" })
# Find the "New Thread" button ref
browser_click_element({ selector: "@e1" })   # ref for "New Thread"
```

Or navigate directly back to the homepage:

```
browser_navigate({ url: "https://www.perplexity.ai" })
```

Both take you back to the clean homepage with an empty search box.

### Follow-up Questions (Same Thread)

To ask a follow-up within the same thread, use the **same submit technique** from Step 1 on the results page. The `#ask-input` box and React fiber structure are identical. The follow-up answer appends below the first answer in the same thread.

To extract the latest answer only, the extraction script already targets `proseBlocks[proseBlocks.length - 1]` — the last (newest) answer block.

---

## Handling Popups

Perplexity occasionally shows a sign-in or upgrade popup on first visit. Dismiss it:

```
browser_scan_page()
# Look for a button with text "Close" or aria-label "Close"
browser_click_element({ selector: "@eN" })   # the Close button ref
```

If no popup appears (common when already logged in), skip this step.

---

## Complete Example Session

```
# 1. Connect (if not already connected)
ConnectMCP(transport: "http", url: "http://localhost:19876/mcp")

# 2. Open Perplexity
browser_navigate({ url: "https://www.perplexity.ai" })

# 3. Dismiss popup if present
browser_scan_page()
# Check for Close button → click if found

# 4. Submit query
browser_execute_script({ code: "...submit script with QUERY..." })

# 5. Confirm navigation
browser_list_tabs()
# URL should now be perplexity.ai/search/...

# 6. Extract answer
browser_execute_script({ code: "...extraction script..." })
# Parse the JSON result

# 7. (Optional) Ask follow-up on same thread
browser_execute_script({ code: "...submit script with FOLLOW_UP_QUERY..." })
browser_execute_script({ code: "...extraction script..." })

# 8. Start fresh for next topic
browser_navigate({ url: "https://www.perplexity.ai" })
```

---

## How It Works Under the Hood

### Why Not Just Type and Press Enter?

Perplexity's search box is a React-managed `contenteditable` `<div>`, not a `<form>` with `<input>`. Standard approaches fail:

| Approach | Why It Fails |
|----------|-------------|
| `browser_fill_form` + Enter keydown | Sets DOM text but React state doesn't update — submit sends empty query |
| `document.execCommand('insertText')` | Doesn't trigger React's synthetic event system |
| Navigate to `/search?q=...` | Works for basic searches but may behave differently than the interactive flow |

### What Does Work

React stores component state and event handlers on fiber nodes attached to DOM elements via `__reactFiber$...` keys. By walking up the fiber tree from `#ask-input`, we find the ancestor component (typically ~28 levels up) that holds `onChange` and `onSubmit` props. Calling these directly updates React state and triggers navigation — identical to a real user typing and pressing Enter.

### MAIN World Requirement

The `__reactFiber$...` properties only exist in the page's MAIN JavaScript world. `browser_execute_script` runs in USER_SCRIPT world (isolated). The `<script>` tag injection pattern bridges this gap.

---

## Tips

- **Check `browser_list_tabs()` after submit** — the URL change from `/` to `/search/...` confirms the query was accepted
- **Retry extraction if `{ "status": "loading" }`** — the AI answer streams in and may take 1–3 seconds to fully render
- **Free tier limits** — Perplexity may show "Free preview limit reached. Now using basic search." which gives shorter answers. Still functional.
- **One tab is enough** — reuse the same tab for all searches; no need to create new tabs
- **Follow-ups are cheap** — they stay in the same thread and share context from the original query
