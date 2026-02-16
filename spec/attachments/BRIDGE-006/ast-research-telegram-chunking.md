# AST Research: Telegram Content-Aware Chunking (BRIDGE-006)

## Research Summary

Analysis of the `bridge/` directory to identify integration points for content-aware chunking.

## Current Architecture

### Core Files

| File | Lines | Purpose |
|------|-------|---------|
| `telegram-endpoint.ts` | 871 | Main WebSocket server + Telegram bot integration |
| `telegram-formatting.ts` | 207 | MarkdownV2 escaping + truncation + chunk formatting |
| `telegram-buffering.ts` | 160 | Buffer state management (extracted but not used) |
| `telegram-slash-commands.ts` | 234 | /help, /status, /stop, /clear handlers |
| `telegram-whitelist.ts` | 130 | User ID validation |

### Key Interfaces

```typescript
// telegram-endpoint.ts:39
interface StreamChunkData {
  type: 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'done' | 'error';
  text?: string;
  thinking?: string;
  name?: string;
  id?: string;
  tool_call_id?: string;
  content?: string;
  is_error?: boolean;
  error?: string;
}

// telegram-endpoint.ts:72
interface EndpointState {
  wss: WebSocketServer | null;
  bot: TelegramBotInstance | null;
  currentSession: { ws: WebSocket | null; sessionId: string | null; };
  chatId: string | null;
  toolNameMap: Map<string, string>;
  isRunning: boolean;
  messageBuffer: string[];        // Current simple buffering
  bufferCharCount: number;
  bufferTimer: ReturnType<typeof setTimeout> | null;
  lastSendTime: number;
  lastChunkTime: number;
  allowedUserIds: Set<number> | null;
  agentState: 'idle' | 'thinking' | 'executing';
}

// telegram-buffering.ts:33
interface BufferState {
  messageBuffer: string[];
  bufferCharCount: number;
  bufferTimer: ReturnType<typeof setTimeout> | null;
  lastSendTime: number;
  lastChunkTime: number;
  bot: TelegramBot | null;
  chatId: string | null;
}
```

### Current Buffering Logic

**telegram-endpoint.ts:94-101 - Configuration constants:**
```typescript
const BUFFER_IDLE_FLUSH_MS = 800;  // Flush after 800ms idle
const MIN_SEND_INTERVAL_MS = 300;  // Rate limiting
const MAX_BUFFER_SIZE = 50;        // Max chunks before force flush
const MAX_BUFFER_CHARS = 3500;     // Force flush before 4096 limit
```

**telegram-endpoint.ts:401-481 - handleStreamChunk():**
- Main entry point for processing outbound messages
- Currently just accumulates text and flushes on:
  - Idle timeout (800ms)
  - Buffer size limits (50 chunks or 3500 chars)
  - Special chunk types (done, error)

**telegram-endpoint.ts:330-371 - flushBuffer():**
- Combines buffer, calls truncateMessage(), sends to Telegram
- No boundary awareness - just joins and truncates

**telegram-formatting.ts:71-106 - truncateMessage():**
- Already handles oversized messages well
- Preserves first/last 1500 chars
- Properly closes/reopens code block fences
- This can be REUSED for code blocks exceeding limit

**telegram-formatting.ts:143-182 - formatForTelegram():**
- Adds emoji prefixes based on chunk type
- Passes content verbatim - NO summarization
- Integration point for content handlers

### Functions to Modify

| Function | File:Line | Modification Needed |
|----------|-----------|---------------------|
| `handleStreamChunk` | telegram-endpoint.ts:401 | Add content-aware chunker before buffering |
| `formatForTelegram` | telegram-endpoint.ts:281 | Add summarization for thinking/tool_result |
| `flushBuffer` | telegram-endpoint.ts:330 | Add boundary detection before flush |

### Functions to Create (telegram-content-chunker.ts)

```typescript
// New module: bridge/telegram-content-chunker.ts

interface ChunkBoundary {
  type: 'sentence' | 'paragraph' | 'heading' | 'code_block' | 'list' | 'max_size';
  position: number;
  priority: number;  // code_block=5, heading=4, paragraph=3, sentence=2, max_size=1
}

// Boundary detection
function findBoundaries(text: string): ChunkBoundary[];
function getBestSplitPoint(text: string, maxPosition: number): number;

// Content summarization
function summarizeThinking(thinking: string): string;
function summarizeToolResult(toolName: string, content: string): string;
function formatToolCall(name: string, args?: Record<string, unknown>): string;

// Markdown validation
function balanceMarkdown(text: string): string;
function isInsideCodeBlock(text: string, position: number): boolean;

// Main chunker
function processChunk(chunk: StreamChunkData, buffer: ContentBuffer): FlushResult;
```

### Integration Points

1. **Entry point**: `handleStreamChunk()` at line 401
   - Currently receives chunks and buffers them
   - Modify to route through content-aware processor

2. **Formatting**: `formatForTelegram()` at line 281
   - Add summarization logic for thinking/tool_result types
   - Keep existing escaping and prefix logic

3. **Flush logic**: `flushBuffer()` at line 330
   - Add boundary detection before flush
   - Split at logical boundaries when over limit

4. **State**: `EndpointState` interface at line 72
   - May need additional state for tracking code block context
   - Track "inside code block" for boundary priority

### Test Files to Create

```
bridge/__tests__/telegram-content-chunker.test.ts
  - Boundary detection tests
  - Summarization tests
  - Markdown validation tests
  - Integration tests with mock buffer
```

### Existing Test Files (for patterns)

- `bridge/__tests__/telegram-endpoint.test.ts` - Main integration tests
- `bridge/__tests__/telegram-slash-commands.test.ts` - Slash command tests
- `bridge/__tests__/telegram-whitelist.test.ts` - Whitelist logic tests

All use Vitest with similar patterns:
- Mock TelegramBot with `sendMessage` spy
- Mock WebSocket with `send` spy
- Test state management via `getState()`, `resetState()`
