# StreamChunk → Scrollback Rendering Matrix (TS Ink reference)

Source of truth for RPC-078. Every cell below was verified by grep across
`src/tui/` for the exact literal — there is NO place in the TS code that
emits the WRONG-PREFIX strings (`user>`, `assistant>`, `[error]`,
`[done]`, `[interrupted]`, `[notice]`, `supervisor>`, `(thinking)`).

| StreamChunk variant     | Visible prefix          | Color       | Multi-line? | Streaming `...` suffix? | TS source |
|-------------------------|-------------------------|-------------|-------------|-------------------------|-----------|
| `UserInput { text }`    | `You: <text>`           | **GREEN**   | no          | no                      | `src/tui/utils/conversationUtils.ts` |
| `Text { text }`         | `● <text>` (U+25CF)     | **WHITE**   | yes         | yes — appended while streaming, stripped on `Done` | `src/tui/utils/conversationUtils.ts`, `src/tui/utils/chunkProcessor.ts` |
| `Thinking { thinking }` | `[Thinking]\n<text>`    | **YELLOW**  | yes         | no                      | `src/tui/utils/thinkingBlockManager.ts` |
| `Error { error }`       | `API Error: <error>`    | **WHITE**   | yes (wraps) | no                      | `src/tui/components/AgentView.tsx` (chunk handler) |
| `Done`                  | *no new line*           | —           | —           | strips trailing `...` from previous Text | `src/tui/utils/chunkProcessor.ts` |
| `Interrupted`           | `⚠ Interrupted`         | **WHITE**   | no          | no                      | `src/tui/components/AgentView.tsx` |
| `Interrupted` *(inside an active tool block)* | `\nL ⚠ Interrupted` | **WHITE** | yes | no | `src/tui/components/AgentView.tsx` |
| `UserNotification { message }` | `<message>` (verbatim) | inherits | yes | no | `src/tui/components/AgentView.tsx:2332` |
| `IncomingMessage { text }`* | `[W] <role>> <body>` | **MAGENTA** | yes | no | `src/tui/utils/conversationUtils.ts` |

\* `IncomingMessage.text` is parsed as
`[SUPERVISOR: <role> | Session: <id>]\n<body>` and rewritten with the
`[W]` prefix.

## Anti-patterns (REJECTED literal strings)

These strings exist nowhere in the TS Ink reference. If a test asserts
any of them, the test is wrong and must be rewritten to assert the
table above instead.

```
user>            (use "You: " green)
assistant>       (use "● " white)
[error]          (use "API Error: " white)
[done]           (no line — strip "..." from previous Text)
[interrupted]    (use "⚠ Interrupted" white)
[notice]         (verbatim — no prefix)
supervisor>      (use "[W] " magenta)
(thinking)       (use "[Thinking]\n" yellow)
```

## Why TS uses these exact prefixes

- `You:` — affirmative voice ("you said X"), matches the chat-UI mental
  model used by every commercial AI tool.
- `●` (U+25CF BLACK CIRCLE) — a single glyph that visually marks an
  assistant turn without consuming horizontal space; high contrast on
  both dark and light terminals.
- `[Thinking]` — explicit label so the user can distinguish reasoning
  from final answer; yellow signals "this is provisional".
- `API Error:` — production-ready phrasing (not debug-style `[error]`),
  so the message reads as a user-facing notice.
- `⚠ Interrupted` — single-glyph + label, no brackets. Brackets are
  reserved for `[Thinking]` and `[W]`.
- `[W]` — short single-letter "Worker" tag, leaves room for the
  worker's role and body on the same line.
