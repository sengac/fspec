# RPC-078 — TypeScript Ink Reference (Source of Truth)

> **Purpose**: Authoritative mapping of every `StreamChunk` variant to the exact prefix string, render color, and source location used by the **TypeScript Ink AgentView**. The Rust ratatui port (`codelet/fspec-tui/src/store/agent_view/`) MUST produce byte-identical scrollback output.

## Verification Method

```bash
# Confirms the Rust prefixes "user>" / "assistant>" / "supervisor>" / "(thinking)"
# / "[done]" / "[notice]" / "[interrupted]" / "[error]" DO NOT EXIST in TS Ink:
grep -RnE 'user>|assistant>|supervisor>|\(thinking\)|\[done\]|\[notice\]|\[interrupted\]|\[error\]' src/tui/
# (empty output — confirms zero occurrences)
```

## Chunk → Prefix / Color Matrix

| StreamChunk variant | Visible prefix             | Color   | TS source (file:line)                                       |
|---------------------|----------------------------|---------|-------------------------------------------------------------|
| `UserInput`         | `You: `                    | GREEN   | `src/tui/utils/conversationUtils.ts:68`                     |
| `Text` (assistant)  | `● ` (U+25CF)              | WHITE   | `src/tui/utils/conversationUtils.ts:70`                     |
| `Text` (streaming)  | `● …text…...`              | WHITE   | `src/tui/utils/conversationUtils.ts:88-90` (trailing "...") |
| `Thinking`          | `[Thinking]\n` + body      | YELLOW  | `src/tui/utils/thinkingBlockManager.ts:36`                  |
| `Error`             | `API Error: {error}`       | WHITE   | `src/tui/utils/chunkProcessor.ts:357,572`                   |
| `Done`              | **no scrollback line**     | —       | `src/tui/utils/chunkProcessor.ts` (only strips trailing `...`) |
| `Interrupted`       | `⚠ Interrupted`            | WHITE   | `src/tui/components/AgentView.tsx:467,3427`                 |
| `Interrupted` (tool active) | `…\nL ⚠ Interrupted` | WHITE   | `src/tui/utils/chunkProcessor.ts:377`                       |
| `UserNotification`  | message verbatim (no prefix) | WHITE | `src/tui/components/AgentView.tsx:469-487`                  |
| `IncomingMessage`   | `[W] {role}> {body}`       | MAGENTA | `src/tui/utils/chunkProcessor.ts:99`, `AgentView.tsx:301`   |
| `HistoryCleared`    | `History cleared`          | WHITE   | `src/tui/utils/chunkProcessor.ts:343`                       |

## Color Map (`renderItem`)

`src/tui/components/AgentView.tsx:5364-5386`:

```tsx
// Thinking content - render in yellow (using isThinking flag)
if (line.isThinking) {
  return <Text color="yellow">{content}</Text>;
}

// Default rendering: user=green, supervisor=magenta, default=white
const baseColor =
  line.role === 'user'
    ? 'green'
    : line.role === 'supervisor'
      ? 'magenta'
      : 'white';
return <Text color={baseColor}>{content}</Text>;
```

## Prefix Construction (`conversationUtils.ts:60-90`)

```ts
// SOLID: Thinking messages get no prefix (the [Thinking] header is already in content)
// WATCH-012: Supervisor messages already have '[W] RoleName>' prefix from processChunksToConversation
const prefix =
  msg.role === 'user'
    ? 'You: '                          // line 68
    : msg.role === 'assistant'
      ? '● '                           // line 70  (U+25CF, BLACK CIRCLE)
      : '';

// While streaming, the LAST line of the assistant message has "..." appended
// (line 88-90). On Done, this trailing "..." is removed — no separate [done] line.
```

## Supervisor Reformat (`chunkProcessor.ts:73-99`)

```ts
// Input chunk: "[SUPERVISOR: reviewer | Session: s-2]\nplease check this"
// Match: /^\[SUPERVISOR: ([^|]+) \| Session: ([^\]]+)\]\n?/
// Output: "[W] reviewer> please check this"
return `[W] ${info.role}> ${info.content}`;
```

## Wrapping Algorithm (`textWrap.ts:72-177`)

```ts
export function wrapText(text: string, options: WrapOptions): string[] {
  // 1. Sanitize ANSI/control chars (sanitizeForTerminal)
  // 2. Normalize emoji widths (normalizeEmojiWidth)
  // 3. Split on '\n' into paragraphs
  // 4. For each paragraph:
  //    - If getVisualWidth(p) <= maxWidth → push as-is
  //    - Else: word-wrap on whitespace
  //    - For words wider than maxWidth: break char-by-char by getVisualWidth
  // 5. Return one string per visual row
}
```

`wrapItems` (line 201-227) calls `wrapText` for each message and emits flat
`WrappedLine[]` so VirtualList's invariant **1 item = 1 visual row** holds.

`calculatePaneWidth` (line 248-269) computes the wrap width from terminal width
minus borders/padding/scrollbar.

## Non-Duplication Invariant

`src/tui/components/AgentView.tsx::handleSubmit` (~line 1870-1873) synchronously
appends ONE `{ type: 'user-input', content: userMessage }` to local conversation
state. Subsequent `StreamChunk::UserInput` broadcasts from the server are
**reconciled, not appended**, because the local message already carries the
text. The Rust port's duplicate stems from BackgroundSession's `send_input`
broadcasting `UserInput` on **every** call (not just `/resume` replay), which
the chunk subscriber then renders a second time.

## Out-of-Scope Decorations (kept WHITE/default)

- `Started`, `Stopped` — TS chunkProcessor does NOT emit visible lines.
- `Connecting`, `Connected`, `Disconnected` — surfaced via `UserNotification`
  (verbatim message), not Debug-style brackets.
- Tool headers (`● ToolName(args)`) — produced by AgentView from
  `tool-use`/`tool-result` chunks, not the variants this card tracks.
