# Persistent Context Implementation in Codelet Multi-Turn System

## Executive Summary

After thorough analysis of the codelet codebase, I've discovered that **codelet does NOT implement traditional persistent context storage**. Instead, it uses an **in-memory context management system** that maintains conversation state only for the duration of a single session. The context is managed through sophisticated in-memory data structures and mechanisms that survive across multiple tool calls within the same session.

## Key Finding: No Disk-Based Persistence

**Critical Discovery**: Codelet maintains all conversation context in RAM during the session lifecycle. There is no file-based persistence, database storage, or session resumption capability across process restarts.

## Architecture Overview

### 1. Core Context Storage Location

The primary context storage is in `/home/rquast/projects/codelet/src/agent/runner.ts`:

```typescript
// Line 594 in runner.ts
let messages: CoreMessage[] = [];
```

This is the **single source of truth** for the entire conversation history during a session.

### 2. Context Lifecycle

#### Session Start (runAgent function)
```typescript
export async function runAgent(): Promise<void> {
  // Initialize provider manager
  let providerManager: ProviderManager = await createProviderManagerAsync();

  // Initialize message history - starts empty
  let messages: CoreMessage[] = [];

  // Initialize token tracking
  const tokenStateManager: TokenStateManager = createTokenStateManager(
    providerManager.getModelName()
  );
  let tokenTracker: TokenUsage = createTokenTracker();

  // Start the REPL loop
  const askQuestion = async () => { /* ... */ };
  void askQuestion();
}
```

#### Session Duration
The `messages` array persists in memory throughout the entire REPL (Read-Eval-Print Loop) session:
- New user messages are pushed to the array
- LLM responses are appended
- System reminders are injected/replaced
- Context is added via deduplication
- Compaction modifies the array in-place when needed

#### Session End
When the user types "exit" or presses Ctrl+C:
```typescript
if (userInput.toLowerCase() === 'exit') {
  inputManager.close();
  return; // Function exits, messages array is garbage collected
}
```

**All context is lost** - no persistence mechanism saves the conversation.

## How Context Survives Tool Calls

### Key Mechanism: Continuous REPL Loop

The magic isn't persistence - it's that the REPL loop never exits during tool execution:

```typescript
// Line 796 in runner.ts
while (continueLoop) {
  // Stream LLM response
  for await (const part of result.fullStream) {
    switch (part.type) {
      case 'tool-call':
        // Tool is executed synchronously within the stream
        toolCalls.push({ toolCallId, toolName, args });
        break;

      case 'tool-result':
        // Result captured immediately
        toolResults.push({ toolCallId, toolName, result });
        break;
    }
  }

  // Add assistant response (including tool calls/results) to history
  const response = await result.response;
  messages.push(...response.messages); // Context preserved!

  // Check finish reason
  if (finishReason === 'tool-calls') {
    // Continue loop - make another LLM call with updated context
    continue;
  }
}
```

### Tool Execution Flow

1. **LLM requests tool**: `finishReason === 'tool-calls'`
2. **Tool executes**: Results captured in `toolResults` array
3. **Response added to messages**: `messages.push(...response.messages)`
4. **Loop continues**: Next iteration includes full history + tool results
5. **LLM sees results**: Because `messages` array contains everything

## Context Management Data Structures

### 1. CoreMessage Type (from Vercel AI SDK)

```typescript
import type { CoreMessage } from 'ai';

interface CoreMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string | Array<ContentPart>;
}

// Example message array structure:
let messages: CoreMessage[] = [
  { role: 'user', content: 'Create a new file' },
  { role: 'assistant', content: [
      { type: 'text', text: 'I will create the file' },
      { type: 'tool-call', toolCallId: '123', toolName: 'Write', args: {...} }
    ]
  },
  { role: 'tool', content: [
      { type: 'tool-result', toolCallId: '123', result: 'File created' }
    ]
  },
  { role: 'assistant', content: 'The file has been created successfully' }
];
```

### 2. Message Array Mutations

The `messages` array is modified throughout the session:

```typescript
// Adding user message
messages.push({
  role: 'user',
  content: userInput,
});

// Adding context with deduplication (line 764)
messages = addContextWithDeduplication(messages, context);

// Adding assistant responses (line 1118)
messages.push(...response.messages);

// Compaction (replaces entire array - line 894-895)
messages.length = 0;
messages.push(...fromCompactionMessages(messagesWithSummary));
```

## Context Preservation Mechanisms

### 1. System Reminders with Deduplication

Location: `/home/rquast/projects/codelet/src/agent/system-reminders.ts`

**Key Innovation**: Type-based deduplication prevents context bloat

```typescript
export function addSystemReminder(
  messages: CoreMessage[],
  type: SystemReminderType,  // 'claudeMd' | 'environment' | 'gitStatus' | 'tokenStatus'
  content: string
): CoreMessage[] {
  // Remove any existing reminders of the same type
  const filteredMessages = removeSystemRemindersByType(messages, type);

  // Create new system reminder with type marker
  const reminderContent = `<system-reminder>
<!-- type:${type} -->
${content}
</system-reminder>`;

  const reminderMessage: CoreMessage = {
    role: 'user',
    content: reminderContent,
  };

  // Prepend the new reminder (following Claude CLI's P1A pattern)
  return [reminderMessage, ...filteredMessages];
}
```

**Purpose**: Each reminder type appears exactly once in the conversation, preventing token waste from repeated context injection.

### 2. Context Injection (CLAUDE.md/AGENTS.md)

Location: `/home/rquast/projects/codelet/src/agent/context.ts`

```typescript
export function addContextWithDeduplication(
  messages: CoreMessage[],
  context: ReturnType<typeof gatherContext>
): CoreMessage[] {
  let updatedMessages = [...messages];

  // Add environment context (replaced if exists)
  const envContent = `Current Directory: ${context.cwd}
Platform: ${context.envInfo.platform}
Architecture: ${context.envInfo.arch}
...`;

  updatedMessages = addSystemReminder(
    updatedMessages,
    'environment',
    envContent
  );

  // Add CLAUDE.md or AGENTS.md content (replaced if exists)
  if (context.claudeMd) {
    const claudeContent = `As you answer the user's questions, you can use the following context:
# CLAUDE.md
${context.claudeMd}
...`;

    updatedMessages = addSystemReminder(
      updatedMessages,
      'claudeMd',
      claudeContent
    );
  }

  return updatedMessages;
}
```

**Context Re-injection**: Every turn, context is re-added using deduplication. Old context is removed, new context replaces it - no duplication.

### 3. Token State Tracking

Location: `/home/rquast/projects/codelet/src/agent/token-state-manager.ts`

```typescript
export interface TokenStateManager {
  modelName: string;
  modelLimits: ModelLimits;
  currentState: TokenUsageState | null;
}

export interface TokenUsageState {
  currentTokens: number;
  totalCapacity: number;
  remainingTokens: number;
  percentageUsed: number;
  modelName: string;
  lastUpdated: Date;
}
```

Tracks token usage across the session to trigger compaction when needed.

### 4. Anchor Point Compaction

Location: `/home/rquast/projects/codelet/src/agent/compaction.ts` and `anchor-point-compaction.ts`

When context exceeds 90% of the model's limit, compaction is triggered:

```typescript
// Line 836 in runner.ts
if (shouldTriggerCompaction(tokenTracker, compactionThreshold)) {
  // Convert messages to compaction format
  const compactionMessages = toCompactionMessages(messages);

  // Perform intelligent compaction with anchor points
  const compactionResult = await compactMessagesWithMetrics(
    compactionMessages,
    summaryProvider,
    budget
  );

  // Replace messages with compacted version + summary
  messages.length = 0;  // Clear array
  messages.push(...fromCompactionMessages(messagesWithSummary));
}
```

**Anchor Point Strategy**: Identifies important conversation points (task completions, error resolutions) and preserves them while summarizing less important turns.

### 5. Interruption Handling

```typescript
// Line 558 in runner.ts
let messagesBeforeInterruption: CoreMessage[] = [];

// Before processing user input (line 749)
messagesBeforeInterruption = [...messages];

// Handle "continue" command (line 673)
if (userInput.toLowerCase() === 'continue' && inputManager.getState().isPaused) {
  // Restore messages to state before interruption
  messages = [...messagesBeforeInterruption];
  userInput = lastUserInput;
}
```

Supports ESC key interruption with resume capability within the same session.

## Token Management

### Token Tracking Data Structure

```typescript
export interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  cachedInputTokens: number;  // Legacy
  reasoningTokens: number;
  totalTokens: number;
  cacheReadInputTokens: number;      // Prompt caching: 90% discount
  cacheCreationInputTokens: number;  // Prompt caching: 25% premium
}
```

### Effective Token Calculation (Accounting for Cache)

```typescript
export function calculateEffectiveTokens(tracker: TokenUsage): number {
  const cacheDiscount = tracker.cacheReadInputTokens * 0.9;
  return tracker.inputTokens - cacheDiscount;
}
```

**Purpose**: Cache reads are 90% cheaper, so compaction isn't needed as aggressively when cache is effective.

## Message Flow Through Multi-Turn Interaction

### Example Flow

```
Session Start
├─ messages = []
├─ User: "Create a test file"
│  └─ messages.push({ role: 'user', content: 'Create a test file' })
│  └─ messages = addContextWithDeduplication(messages, context)
│     └─ Injects: environment reminder, CLAUDE.md reminder
│
├─ LLM Response (with tool call)
│  └─ messages.push({ role: 'assistant', content: [text, tool-call] })
│  └─ finishReason: 'tool-calls' → loop continues
│
├─ Tool Result
│  └─ messages.push({ role: 'tool', content: [tool-result] })
│
├─ LLM Response (acknowledging tool result)
│  └─ messages.push({ role: 'assistant', content: 'File created' })
│  └─ finishReason: 'end_turn' → loop exits
│
├─ User: "Add content to the file"
│  └─ messages.push({ role: 'user', content: 'Add content...' })
│  └─ Context re-injected (replaces old context via deduplication)
│
├─ ... conversation continues ...
│
└─ Compaction triggers at 90% token limit
   └─ messages = compacted + summary
   └─ Conversation continues with reduced context
```

## No Persistence - Why This Works

### Session-Scoped Design

Codelet is designed as a **session-scoped REPL**:
- Each session is independent
- No need to resume previous sessions
- User starts fresh each time
- This is the same model as `python` REPL or `node` REPL

### Advantages

1. **Simplicity**: No database, no file I/O, no corruption issues
2. **Performance**: In-memory operations are extremely fast
3. **Privacy**: No conversation history on disk
4. **Statefulness**: Within session, full context is available

### Limitations

1. **No Resume**: Cannot continue previous conversations after restart
2. **Memory Bound**: Large conversations consume RAM
3. **No History**: No conversation search or replay
4. **Single Session**: Cannot run multiple independent conversations

## Implementation Details for codelet

### Critical Components to Replicate

1. **Message Array Management**
   ```rust
   // In your runner/agent module
   struct AgentSession {
       messages: Vec<CoreMessage>,
       token_tracker: TokenUsage,
       token_state_manager: TokenStateManager,
       // ... other state
   }
   ```

2. **System Reminder Deduplication**
   ```rust
   fn add_system_reminder(
       messages: Vec<CoreMessage>,
       reminder_type: SystemReminderType,
       content: String
   ) -> Vec<CoreMessage> {
       // Remove existing reminders of same type
       let filtered: Vec<_> = messages.into_iter()
           .filter(|msg| !is_system_reminder_of_type(msg, &reminder_type))
           .collect();

       // Prepend new reminder
       let reminder = create_system_reminder(reminder_type, content);
       let mut result = vec![reminder];
       result.extend(filtered);
       result
   }
   ```

3. **REPL Loop with Tool Execution**
   ```rust
   async fn run_agent() -> Result<()> {
       let mut messages: Vec<CoreMessage> = vec![];

       loop {
           let user_input = read_user_input().await?;
           if user_input == "exit" { break; }

           // Add user message
           messages.push(CoreMessage::user(user_input));

           // Add context with deduplication
           messages = add_context_with_deduplication(messages, gather_context());

           // Inner loop for tool execution
           let mut continue_loop = true;
           while continue_loop {
               let response = stream_llm_response(&messages).await?;

               // Add assistant response to messages
               messages.extend(response.messages);

               // Check finish reason
               continue_loop = matches!(response.finish_reason, FinishReason::ToolCalls);
           }
       }

       Ok(())
   }
   ```

4. **Compaction Trigger**
   ```rust
   // Before each LLM call
   if should_trigger_compaction(&token_tracker, compaction_threshold) {
       let compacted = compact_messages_with_metrics(
           &messages,
           &summary_provider,
           budget
       ).await?;

       messages = compacted.messages;
   }
   ```

5. **Context Injection on Each Turn**
   ```rust
   // Every user input triggers context re-injection
   messages.push(CoreMessage::user(user_input));

   // THEN add context (this replaces old context via deduplication)
   let context = gather_context();
   messages = add_context_with_deduplication(messages, context);
   ```

## Data Structures Reference

### CoreMessage (Vercel AI SDK format)

```typescript
type CoreMessage = {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string | Array<ContentPart>;
};

type ContentPart =
  | { type: 'text'; text: string }
  | { type: 'tool-call'; toolCallId: string; toolName: string; args: unknown }
  | { type: 'tool-result'; toolCallId: string; result: unknown };
```

### MessageForCompaction

```typescript
interface MessageForCompaction {
  role: 'user' | 'assistant' | 'system';
  content: string;
  tokens: number;
  isSystem?: boolean;
  hasActiveToolCall?: boolean;
  isToolResult?: boolean;
  toolCallIds?: string[];
  originalContent?: string | Array<unknown>;  // Preserves structure during compaction
}
```

## Summary: How to Replicate in codelet

### Core Architecture

1. **Single Message Vec**: Maintain `Vec<CoreMessage>` in `run_agent()` function scope
2. **REPL Loop**: Never exit the loop during tool execution
3. **Message Accumulation**: Push all messages (user, assistant, tool) to the vec
4. **System Reminder Deduplication**: Implement type-based replacement logic
5. **Context Re-injection**: Every turn, re-add context using deduplication
6. **Compaction**: When tokens exceed threshold, compact messages in-place
7. **No Persistence**: Don't implement save/load - keep it session-scoped

### Key Insight

The "magic" of persistent context isn't persistence at all - it's that:
1. The message array lives in the function scope of `run_agent()`
2. The REPL loop (`askQuestion`) is a closure that captures `messages`
3. Tool execution happens within the streaming loop, not as separate calls
4. Messages accumulate in the array throughout the session
5. Compaction modifies the array in-place when needed

### What NOT to Build

- ❌ Database storage
- ❌ File-based session persistence
- ❌ Session resume capability
- ❌ Conversation history browser
- ❌ Multi-session management

### What TO Build

- ✅ In-memory message Vec
- ✅ REPL loop with closure over messages
- ✅ System reminder deduplication by type
- ✅ Context re-injection each turn
- ✅ Token tracking and compaction
- ✅ Tool execution within streaming loop
- ✅ Message accumulation pattern

The elegance of codelet's design is its simplicity: everything is in RAM, there's a single loop, and context survives because the function never returns until the user exits.

---

**Report Generated**: 2025-12-03
**Codebase**: /home/rquast/projects/codelet
**Analysis Scope**: Persistent context architecture, in-memory message management, session lifecycle
