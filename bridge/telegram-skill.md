# Telegram Bridge Connection

Connect this AI session to a Telegram bridge endpoint for remote monitoring and interaction.

## Default Configuration

- **WebSocket URL**: `ws://localhost:8080`

## Action Required

When this skill is referenced, immediately connect to the Telegram bridge using the Bridge tool.

If a custom URL is provided as a parameter, use that instead of the default.

## How to Use This Skill

Reference this file with `@` in your prompt to load the skill:

```
@bridge/telegram-skill.md                          # Connect to ws://localhost:8080
@bridge/telegram-skill.md ws://myserver:9000       # Connect to custom URL
```

---

## Execute Now

Connect to the Telegram bridge at `ws://localhost:8080`:
