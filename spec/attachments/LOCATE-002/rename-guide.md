# LOCATE-002: Extension Rename & Rebrand Guide

## Files to Update

### Core Extension Files

| File | Field/Reference | Current Value | New Value |
|------|----------------|---------------|-----------|
| `extension/manifest.json` | `name` | `"fspec WebMCP Bridge"` | `"fspec Browser Agent"` (or chosen name) |
| `extension/package.json` | `name` | `"fspec-webmcp-extension"` | `"fspec-browser-agent"` |
| `extension/package.json` | `description` | `"fspec WebMCP Chrome Extension - Bridge WebMCP tools to MCP for AI agent interaction"` | Updated description |
| `extension/popup.html` | `<title>` | `"fspec WebMCP Bridge"` | New name |
| `extension/popup.html` | `<h1>` | `"🔗 fspec WebMCP Bridge"` | New name with icon |

### Documentation Files

| File | References |
|------|-----------|
| `extension/webmcp-skill.md` | Multiple references to "fspec WebMCP Chrome Extension", "WebMCP extension", "fspec WebMCP Bridge" |
| `extension/inject-webmcp-tools-skill.md` | References to "fspec WebMCP Chrome Extension", "fspec WebMCP Bridge" |

### Source Code Headers

| File | Header Comment |
|------|---------------|
| `extension/src/background/browser-tools.ts` | `"fspec WebMCP Extension - Native Browser Control Tools"` |
| `extension/src/background/browser-events.ts` | `"fspec WebMCP Extension - Browser Event Listeners"` |
| `extension/src/background/service-worker.ts` | `"fspec WebMCP Extension - Service Worker"` |
| `extension/src/background/message-router.ts` | Check for old name references |
| `extension/src/background/native-connection.ts` | Check for old name references |

### Error Messages

Search for and update:
- `"fspec WebMCP Bridge → Details → Allow User Scripts"` in browser-tools.ts
- Any native host references in `extension/host/native-host.mjs`

### MCP Server

| File | Reference |
|------|-----------|
| `extension/host/lib/mcp-server.mjs` | `SERVER_NAME = 'fspec-webmcp'` → update |

## Name Suggestions Analysis

### Top 3 Recommendations

1. **fspec Browser Agent** ⭐ RECOMMENDED
   - Matches the most popular naming pattern in the ecosystem ("Browser Agent Extension" on Chrome Web Store)
   - Clear, descriptive, professional
   - Aligns with what the extension does: it's an agent that controls the browser
   - `package.json` name: `fspec-browser-agent`

2. **fspec Browser Companion**
   - Warmer, implies a helpful AI partner
   - Less "techy" — good for broader audience
   - Could be perceived as less capable than "agent"
   - `package.json` name: `fspec-browser-companion`

3. **fspec Browser Connect**
   - Clean, professional, emphasizes the MCP connection
   - More generic — doesn't convey the AI agent capabilities
   - `package.json` name: `fspec-browser-connect`

### Rejected Alternatives

- **fspec Browser Pilot** — "Pilot" has confusing connotations (auto-pilot vs co-pilot)
- **fspec Browser Lens** — Too narrow, suggests viewing but not interacting
- **fspec for Chrome** — Too limiting (extension may support Firefox/Edge later)
- **fspec BrowserMCP** — Too technical, focuses on protocol not capability
- **fspec Browser Bridge** — Current name, too passive for the expanding scope

### Ecosystem Context

| Extension | Stars/Users | Naming Pattern |
|-----------|------------|----------------|
| Browser Agent Extension (Chrome Web Store) | Popular MCP bridge | "{Product} Agent Extension" |
| Chrome MCP Server (hangwin/mcp-chrome) | 1k+ stars | Protocol-focused name |
| agent-browser (Vercel Labs) | Active | "agent-{platform}" |
| browser-use | ~80k stars | Action-verb name |

## Verification Checklist

After renaming, verify:
- [ ] `npm run build` succeeds in extension/
- [ ] Extension loads in Chrome without errors
- [ ] Popup displays new name
- [ ] `chrome://extensions` shows new name
- [ ] Native host still connects (check host manifest)
- [ ] Skill files reference new name consistently
- [ ] `grep -ri "WebMCP Bridge" extension/` returns 0 results
- [ ] `grep -ri "fspec-webmcp-extension" extension/` returns 0 results
