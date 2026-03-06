# LOCATE-008: MCP Tool Definitions & Skill Documentation — Checklist

## 1. MCP Server Tool Definitions

**File:** `extension/host/lib/mcp-server.mjs`

Add to `NATIVE_TOOLS` array:

```javascript
{
  name: 'browser_scan_page',
  description: 'Scan the page for interactive elements, returning an accessibility tree with ref labels. Use refs (@e1, @e2) with browser_click_element and browser_fill_form.',
  inputSchema: {
    type: 'object',
    properties: {
      tabId: { type: 'number', description: 'Tab to scan (defaults to active tab)' },
      interactive: { type: 'boolean', description: 'Only show interactive elements (default: true)' },
      selector: { type: 'string', description: 'CSS selector to scope the scan to a subtree' },
    },
  },
},
{
  name: 'browser_diff_page',
  description: 'Show what changed on the page since the last browser_scan_page call. Returns a unified diff with additions and removals.',
  inputSchema: {
    type: 'object',
    properties: {
      tabId: { type: 'number', description: 'Tab to diff (defaults to active tab)' },
    },
  },
},
```

## 2. Skill Documentation Updates

**File:** `extension/webmcp-skill.md`

### Header Update
- Change tool count from 12 to 14
- Add mention of scan/ref/diff capabilities

### New Tool Sections

Add documentation for `browser_scan_page` and `browser_diff_page`:

```markdown
### `browser_scan_page`
Scan the active tab for interactive elements. Returns an accessibility-tree-like 
representation with ref labels that can be used with browser_click_element and 
browser_fill_form.

### `browser_diff_page`  
Show what changed on the page since the last scan. Returns a unified diff.
```

### Updated Tool Sections

Update `browser_click_element` and `browser_fill_form` docs to mention @ref syntax:

```markdown
- `selector` (string, **required**): CSS selector OR ref (e.g., `@e3` from browser_scan_page)
```

### New Common Workflows Section

Add the scan→interact→verify workflow:

```markdown
### Interact with page elements using refs
1. browser_navigate → URL
2. browser_scan_page → get tree with @e1, @e2, @e3 refs
3. browser_fill_form → { selector: "@e1", value: "user@test.com" }
4. browser_fill_form → { selector: "@e2", value: "password123" }
5. browser_click_element → { selector: "@e3" }
6. browser_diff_page → verify what changed
7. browser_scan_page → re-scan after navigation
```

### Ref Lifecycle Section

```markdown
## Ref Lifecycle
- Refs are assigned when you call `browser_scan_page`
- Refs are **ephemeral** — they're invalidated when the page navigates
- Always re-scan after navigation or significant page changes
- If a ref is not found, you'll get an error suggesting to re-scan
```

## 3. Inject-WebMCP Skill Updates

**File:** `extension/inject-webmcp-tools-skill.md`

- Update any references to old extension name
- No functional changes needed (this file is about WebMCP tool injection, not native tools)

## 4. Popup HTML

**File:** `extension/popup.html`

- Update tool count display if hardcoded
- Note: Tool count is dynamically retrieved via GET_STATUS message, so may not need updating

## 5. Verification Checklist

After documentation changes:
- [ ] webmcp-skill.md renders correctly in markdown preview
- [ ] All 14 native tools are listed consistently
- [ ] @ref syntax is documented in click and fill tool sections
- [ ] Common workflows section includes scan→interact→verify
- [ ] Troubleshooting section updated for ref-related errors
- [ ] inject-webmcp-tools-skill.md has no stale name references
- [ ] MCP server NATIVE_TOOLS has correct inputSchema for both new tools
- [ ] `tools/list` call returns all 14 tools (manual verification)
