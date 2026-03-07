# AST Research: MCP Server Tool Definitions

## NATIVE_TOOLS Analysis

The `NATIVE_TOOLS` array in `extension/host/lib/mcp-server.mjs` already contains both new tool definitions:

### browser_scan_page (line 157)
- **name**: `browser_scan_page`
- **description**: Scan the page DOM and build an accessibility-tree-like representation with interactive element refs
- **inputSchema properties**: `tabId` (number), `interactive` (boolean), `selector` (string)
- ✅ Already present — no code changes needed

### browser_diff_page (line 169)
- **name**: `browser_diff_page`
- **description**: Show what changed on the page since the last browser_scan_page call
- **inputSchema properties**: `tabId` (number)
- ✅ Already present — no code changes needed

### Total NATIVE_TOOLS count: 14
Tools: browser_navigate, browser_screenshot, browser_list_tabs, browser_execute_script, browser_switch_tab, browser_close_tab, browser_get_page_content, browser_click_element, browser_fill_form, browser_go_back, browser_go_forward, browser_create_tab, browser_scan_page, browser_diff_page

## webmcp-skill.md Status
- Currently shows "12 native browser control tools" (stale)
- Missing documentation sections for browser_scan_page and browser_diff_page
- Missing @ref syntax in click/fill tool docs
- Missing scan→interact→verify workflow
- Missing Ref Lifecycle section
- Missing ref-related troubleshooting

## inject-webmcp-tools-skill.md Status
- Already uses "fspec Browser Agent" naming
- No stale references found

## popup.html Status
- Tool count is dynamic (via GET_STATUS message) — no changes needed
