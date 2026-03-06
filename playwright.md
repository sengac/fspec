# Playwright MCP Integration

## Overview

This guide explains how to connect to the [Playwright MCP server](https://github.com/microsoft/playwright-mcp) to gain browser automation capabilities directly within your AI agent session. Once connected, you can interact with web pages, fill forms, click elements, take screenshots, and more — all through MCP tool calls.

---

## Connecting to the Playwright MCP Server

Use the **ConnectMCP** tool with the following parameters:

| Parameter   | Value                      |
|-------------|----------------------------|
| name        | `playwright`               |
| transport   | `stdio`                    |
| command     | `npx @playwright/mcp@latest` |

### MCP Server JSON Configuration

This is the equivalent MCP JSON configuration for reference:

```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": [
        "@playwright/mcp@latest"
      ]
    }
  }
}
```

### ConnectMCP Tool Call

To establish the connection, call the `ConnectMCP` tool with these arguments:

- **action**: `connect`
- **name**: `playwright`
- **transport**: `stdio`
- **command**: `npx @playwright/mcp@latest`

---

## Available Tools After Connection

Once connected, Playwright MCP tools become available with the `mcp__playwright__` prefix. Common tools include:

| Tool                                | Description                          |
|-------------------------------------|--------------------------------------|
| `mcp__playwright__browser_navigate` | Navigate to a URL                    |
| `mcp__playwright__browser_snapshot` | Capture an accessibility snapshot    |
| `mcp__playwright__browser_click`    | Click an element by ref              |
| `mcp__playwright__browser_fill`     | Fill a form field                    |
| `mcp__playwright__browser_screenshot` | Take a screenshot                  |
| `mcp__playwright__browser_go_back`  | Navigate back                        |
| `mcp__playwright__browser_go_forward` | Navigate forward                   |
| `mcp__playwright__browser_select_option` | Select a dropdown option        |
| `mcp__playwright__browser_hover`    | Hover over an element                |
| `mcp__playwright__browser_type`     | Type text character by character     |
| `mcp__playwright__browser_press_key` | Press a keyboard key                |
| `mcp__playwright__browser_wait`     | Wait for a specified duration        |
| `mcp__playwright__browser_close`    | Close the browser                    |

> **Note:** The exact list of available tools may vary by version. After connecting, you can use `ConnectMCP` with `action: list` to see active connections.

---

## Usage Workflow

### 1. Connect

Call `ConnectMCP` to start the Playwright MCP server.

### 2. Navigate

Use `mcp__playwright__browser_navigate` with a `url` parameter to open a page.

### 3. Inspect

Use `mcp__playwright__browser_snapshot` to get an accessibility tree of the page. This returns element references (refs) that you use for interactions.

### 4. Interact

Use tools like `mcp__playwright__browser_click`, `mcp__playwright__browser_fill`, and `mcp__playwright__browser_select_option` with the `ref` values from the snapshot.

### 5. Verify

Use `mcp__playwright__browser_screenshot` to visually confirm the state of the page.

### 6. Disconnect

When done, call `ConnectMCP` with `action: disconnect` and `name: playwright` to tear down the connection.

---

## Tips

- **Always snapshot before interacting** — you need element `ref` values from the accessibility snapshot to click or fill elements.
- **Screenshots are useful for verification** — use them to confirm visual state after interactions.
- **The browser persists across calls** — you don't need to reconnect between page interactions within the same session.
- **npx handles installation** — the `npx @playwright/mcp@latest` command will automatically download the latest version if not already cached.
