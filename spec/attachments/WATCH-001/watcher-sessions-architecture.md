# Watcher Sessions Architecture

## Overview

Watcher sessions enable **multiple AI agents to observe and interact with a parent session** in real-time. Each watcher is a fully interactive session that:
- Observes the parent's streaming output (user input, assistant responses, tool calls/results)
- Has autonomy over its own context management (decides what to keep/discard)
- Can interject into the parent session (interrupt or wait for turn completion)
- Accepts its own user input (to update its "watching brief" or give it new instructions)

## Key Concepts

### Session Roles

Every session has a **role** that defines how other sessions perceive its messages:
- Defined at session start (restored from disk OR interactively defined with user)
- Determines authority level (peer vs supervisor)
- Examples: "security reviewer", "test coverage enforcer", "architecture advisor", "brainstorming partner"

### Watcher Brief

A watcher's "brief" is its initial instructions for what to watch for and when to interject:
- Set when watch relationship is established
- Can be updated by the watcher's user during the session
- Watcher AI continuously evaluates parent stream against its brief

### Message Types

Extend the existing message model with a new type:

```
user      → green text   (parent's human user)
assistant → white text   (AI response)
watcher   → purple text  (watcher AI injection)
```

Watcher messages include:
- `watcher_session_id`: identifies which watcher sent it
- `watcher_role`: the watcher's defined role (for parent AI context)

## Architecture

### Current State (Before)

```
┌─────────────────────────────────────────────────────────────┐
│                     SessionManager                          │
│                       (singleton)                           │
│  ┌─────────────────────────────────────────────────────┐   │
│  │     sessions: HashMap<Uuid, Arc<BackgroundSession>> │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
     │  Session A  │  │  Session B  │  │  Session C  │
     │  (isolated) │  │  (isolated) │  │  (isolated) │
     └─────────────┘  └─────────────┘  └─────────────┘
```

Each session is **isolated** - no communication between sessions.

### Proposed State (After)

```
┌─────────────────────────────────────────────────────────────┐
│                     SessionManager                          │
│                       (singleton)                           │
│  ┌─────────────────────────────────────────────────────┐   │
│  │     sessions: HashMap<Uuid, Arc<BackgroundSession>> │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │     watch_graph: WatchGraph                         │   │
│  │       - parent_to_watchers: HashMap<Uuid, Vec<Uuid>>│   │
│  │       - watcher_to_parent: HashMap<Uuid, Uuid>      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
     ┌─────────────────┐             ┌─────────────────┐
     │  Parent Session │◄────────────│ Watcher Session │
     │                 │  subscribes │                 │
     │  broadcast_tx ──┼─────────────┼→ broadcast_rx   │
     │                 │             │                 │
     │  input_tx ◄─────┼─────────────┼─ injects via    │
     │                 │             │  WatcherInput   │
     └─────────────────┘             └─────────────────┘
```

### Component Changes

#### 1. BackgroundSession Additions

```rust
pub struct BackgroundSession {
    // ... existing fields ...
    
    /// Session role (defines how other sessions perceive this session's messages)
    pub role: RwLock<SessionRole>,
    
    /// Broadcast channel for watchers to subscribe to output stream
    /// Capacity sized for real-time streaming without backpressure issues
    broadcast_tx: broadcast::Sender<StreamChunk>,
    
    /// Optional parent session ID (if this is a watcher)
    parent_session_id: RwLock<Option<Uuid>>,
}

pub struct SessionRole {
    /// Human-readable role name (e.g., "Security Reviewer")
    pub name: String,
    /// Role description/brief
    pub description: String,
    /// Authority level: Peer or Supervisor
    pub authority: RoleAuthority,
}

pub enum RoleAuthority {
    /// Suggestions that parent AI can consider
    Peer,
    /// Instructions that parent AI should follow
    Supervisor,
}
```

#### 2. Input Channel Extension

```rust
/// Input message sent to the agent loop via channel
pub enum SessionInput {
    /// Regular user prompt
    UserPrompt(PromptInput),
    
    /// Injection from a watcher session
    WatcherInput {
        /// The watcher's session ID
        watcher_session_id: Uuid,
        /// The watcher's role name
        watcher_role: String,
        /// The watcher's authority level
        authority: RoleAuthority,
        /// The injected message content
        content: String,
        /// Whether to interrupt current response or wait
        interrupt: bool,
    },
}
```

#### 3. StreamChunk Extension

```rust
/// Add new chunk type for watcher messages
impl StreamChunk {
    pub fn watcher_input(
        watcher_session_id: String,
        watcher_role: String,
        content: String,
    ) -> Self {
        Self {
            chunk_type: "WatcherInput".to_string(),
            text: Some(content),
            watcher_info: Some(WatcherInfo {
                session_id: watcher_session_id,
                role: watcher_role,
            }),
            // ... other fields None ...
        }
    }
}

#[napi(object)]
pub struct WatcherInfo {
    pub session_id: String,
    pub role: String,
}
```

#### 4. WatchGraph

```rust
/// Tracks parent-watcher relationships
pub struct WatchGraph {
    /// Parent session ID → list of watcher session IDs
    parent_to_watchers: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Watcher session ID → parent session ID
    watcher_to_parent: RwLock<HashMap<Uuid, Uuid>>,
}

impl WatchGraph {
    /// Register a watcher for a parent session
    pub fn add_watcher(&self, parent_id: Uuid, watcher_id: Uuid);
    
    /// Remove a watcher relationship
    pub fn remove_watcher(&self, watcher_id: Uuid);
    
    /// Get all watchers for a parent
    pub fn get_watchers(&self, parent_id: Uuid) -> Vec<Uuid>;
    
    /// Get parent for a watcher (if any)
    pub fn get_parent(&self, watcher_id: Uuid) -> Option<Uuid>;
}
```

#### 5. handle_output() Modification

```rust
impl BackgroundSession {
    pub fn handle_output(&self, chunk: StreamChunk) {
        // 1. Always buffer (existing behavior)
        {
            let mut buffer = self.output_buffer.write().unwrap();
            buffer.push(chunk.clone());
        }
        
        // 2. Forward to attached UI callback (existing behavior)
        if self.is_attached() {
            if let Some(cb) = self.attached_callback.read().unwrap().as_ref() {
                let _ = cb.call(Ok(chunk.clone()), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }
        
        // 3. NEW: Broadcast to watchers
        // broadcast::send() is non-blocking and handles no-receiver case gracefully
        let _ = self.broadcast_tx.send(chunk);
    }
}
```

### Data Flow

#### Parent → Watcher (Observation)

```
Parent Session                              Watcher Session
     │                                            │
     │  agent produces StreamChunk                │
     ▼                                            │
handle_output()                                   │
     │                                            │
     ├─► buffer (for replay)                      │
     │                                            │
     ├─► UI callback (if attached)                │
     │                                            │
     └─► broadcast_tx.send(chunk) ───────────────►│ broadcast_rx.recv()
                                                  │
                                                  ▼
                                         Watcher's agent loop
                                         evaluates chunk against
                                         watching brief
                                                  │
                                                  ▼
                                         Watcher decides whether
                                         to keep in context or
                                         discard (context mgmt)
```

#### Watcher → Parent (Injection)

```
Watcher Session                              Parent Session
     │                                            │
     │  Watcher AI decides to interject           │
     ▼                                            │
WatcherOutput::inject()                           │
     │                                            │
     │  Creates WatcherInput message              │
     │                                            │
     └─► parent.input_tx.send(                   ─┼─► input_rx.recv()
           SessionInput::WatcherInput {           │
             watcher_session_id,                  │
             watcher_role,                        │
             authority,                           │
             content,                             ▼
             interrupt,                    agent_loop processes
           }                               WatcherInput:
         )                                        │
                                                  ├─► if interrupt: set is_interrupted
                                                  │
                                                  ├─► add to messages as watcher message
                                                  │
                                                  └─► broadcast WatcherInput chunk
                                                      (so UI shows purple text)
```

### NAPI Bindings

```rust
/// Create a watcher session that observes a parent session
#[napi]
pub async fn session_create_watcher(
    parent_session_id: String,
    model: String,
    role_name: String,
    role_description: String,
    authority: String,  // "peer" or "supervisor"
) -> Result<String>;

/// Get the parent session ID for a watcher (if any)
#[napi]
pub fn session_get_parent(session_id: String) -> Result<Option<String>>;

/// Get all watcher session IDs for a parent
#[napi]
pub fn session_get_watchers(session_id: String) -> Result<Vec<String>>;

/// Inject a message from watcher into parent session
#[napi]
pub fn watcher_inject(
    watcher_session_id: String,
    content: String,
    interrupt: bool,
) -> Result<()>;

/// Set/update session role
#[napi]
pub fn session_set_role(
    session_id: String,
    role_name: String,
    role_description: String,
    authority: String,
) -> Result<()>;

/// Get session role
#[napi]
pub fn session_get_role(session_id: String) -> Result<SessionRoleInfo>;
```

### TypeScript/UI Changes

#### StreamChunk Type Extension

```typescript
interface StreamChunk {
  type: 'Text' | 'Thinking' | 'ToolCall' | 'ToolResult' | 'ToolProgress' 
      | 'Status' | 'Interrupted' | 'TokenUpdate' | 'ContextFillUpdate' 
      | 'Done' | 'Error' | 'UserInput' 
      | 'WatcherInput';  // NEW
  
  // ... existing fields ...
  
  // NEW: Watcher info (present when type === 'WatcherInput')
  watcherInfo?: {
    sessionId: string;
    role: string;
  };
}
```

#### Color Coding in AgentView.tsx

```typescript
// In renderLine or equivalent:
const baseColor = (() => {
  switch (line.role) {
    case 'user': return 'green';
    case 'watcher': return 'magenta';  // Purple for watcher input
    case 'assistant': return 'white';
    default: return 'white';
  }
})();
```

### Context Management for Watchers

Watchers need autonomy over their context to avoid blowing their context window:

```rust
/// Watcher-specific configuration
pub struct WatcherConfig {
    /// Maximum tokens to retain from observed stream
    pub max_observed_tokens: u64,
    
    /// Strategy for what to keep
    pub retention_strategy: RetentionStrategy,
}

pub enum RetentionStrategy {
    /// Keep most recent N tokens
    SlidingWindow { tokens: u64 },
    
    /// Keep based on relevance to watching brief
    RelevanceFiltered,
    
    /// Let watcher AI decide what to keep
    AiManaged,
}
```

The watcher's agent loop can then filter/summarize observed content before adding to its own context.

### Persistence Considerations

When persisting sessions:
- Store `role` with session metadata
- Store `parent_session_id` for watchers
- On restore, re-establish watch relationships via `WatchGraph`
- Watchers can replay parent's buffered output on reattach

### Watcher Agent Loop: Dual-Input Design

A watcher session runs a modified agent loop that handles **two input sources**:

1. **User prompts** - the watcher's own user interacting directly
2. **Parent observations** - accumulated stream chunks from the parent at natural breakpoints

```rust
/// Watcher-specific agent loop
async fn watcher_agent_loop(
    session: Arc<BackgroundSession>,
    mut user_input_rx: mpsc::Receiver<PromptInput>,
    mut parent_broadcast_rx: broadcast::Receiver<StreamChunk>,
    watcher_config: WatcherConfig,
) {
    let mut observation_buffer: Vec<StreamChunk> = Vec::new();
    
    loop {
        tokio::select! {
            // Source 1: Direct user input to watcher
            Some(prompt) = user_input_rx.recv() => {
                // Normal agent handling - user talking to watcher
                session.set_status(SessionStatus::Running);
                session.reset_interrupt();
                
                let output = BackgroundOutput::new(session.clone());
                run_agent_stream(agent, &prompt.input, ..., &output).await;
                
                session.set_status(SessionStatus::Idle);
            }
            
            // Source 2: Parent stream observation
            Ok(chunk) = parent_broadcast_rx.recv() => {
                // Accumulate observation
                observation_buffer.push(chunk.clone());
                
                // Also emit to watcher's own output buffer for UI display
                // (so watcher UI shows what it's observing)
                session.handle_output(StreamChunk::observation(chunk.clone()));
                
                // Check if we've reached a natural evaluation point
                if is_observation_trigger(&chunk) && !observation_buffer.is_empty() {
                    // Format observations for evaluation
                    let eval_prompt = format_evaluation_prompt(
                        &observation_buffer,
                        &watcher_config.brief,
                        &watcher_config.role_name,
                    );
                    
                    // Run agent evaluation (watcher AI decides whether to interject)
                    session.set_status(SessionStatus::Running);
                    let eval_output = EvaluationOutput::new(session.clone());
                    run_agent_stream(agent, &eval_prompt, ..., &eval_output).await;
                    session.set_status(SessionStatus::Idle);
                    
                    // Parse evaluation result for interjection decision
                    if let Some(interjection) = eval_output.get_interjection() {
                        // Inject into parent session
                        let parent = SessionManager::instance()
                            .get_session(&watcher_config.parent_session_id)?;
                        parent.receive_watcher_input(WatcherInput {
                            watcher_session_id: session.id,
                            watcher_role: watcher_config.role_name.clone(),
                            authority: watcher_config.authority,
                            content: interjection.content,
                            interrupt: interjection.is_urgent,
                        });
                    }
                    
                    // Watcher decides what to keep in context (context management autonomy)
                    observation_buffer = watcher_config
                        .retention_strategy
                        .filter(observation_buffer);
                }
            }
        }
    }
}
```

### Observation Triggers (Natural Breakpoints)

The watcher evaluates accumulated observations at **meaningful moments**:

| Chunk Type | Triggers Evaluation? | Rationale |
|------------|---------------------|-----------|
| `Done` | ✅ **Yes** | Assistant turn complete - primary checkpoint |
| `ToolResult` | ✅ **Yes** | Tool finished executing - good evaluation point |
| `Error` | ✅ **Yes** | Something went wrong - may need intervention |
| `Interrupted` | ✅ **Yes** | User interrupted - context changed |
| `Text` | ❌ No | Mid-stream, accumulate and wait |
| `ToolCall` | ❌ No | Wait for tool result |
| `Thinking` | ❌ No | Internal reasoning, wait for output |
| `TokenUpdate` | ❌ No | Metadata only |

```rust
fn is_observation_trigger(chunk: &StreamChunk) -> bool {
    matches!(
        chunk.chunk_type.as_str(),
        "Done" | "ToolResult" | "Error" | "Interrupted"
    )
}
```

### Evaluation Prompt Format

When a trigger occurs, the watcher AI is prompted to evaluate:

```
[WATCHER EVALUATION]

You are "Security Reviewer" watching "Main Development Session".
Your authority level: Supervisor (your interjections should be followed)

YOUR WATCHING BRIEF:
---
Watch for security vulnerabilities including:
- SQL injection, XSS, command injection  
- Exposed credentials or API keys
- Unsafe file operations
- Insecure cryptographic practices

Interrupt immediately for critical security issues.
---

RECENT ACTIVITY IN PARENT SESSION:
---
[User]: Can you help me write a login function?

[Assistant]: I'll create a login function for you:

```javascript
function login(username, password) {
  const query = `SELECT * FROM users WHERE 
    username='${username}' AND password='${password}'`;
  return db.query(query);
}
```

[ToolCall]: Write { file: "auth.js", content: "..." }
[ToolResult]: Successfully wrote auth.js
---

EVALUATE AND DECIDE:
Based on your watching brief, should you interject?

If YES, respond with:
[INTERJECT]
urgent: true/false
content: Your message to the parent session
[/INTERJECT]

If NO (nothing concerning), respond with:
[CONTINUE]
Brief note on what you observed (for your own context)
[/CONTINUE]
```

### Interjection Parsing

```rust
struct Interjection {
    is_urgent: bool,  // If true, interrupt parent mid-stream
    content: String,  // Message to inject into parent
}

fn parse_interjection(response: &str) -> Option<Interjection> {
    if let Some(start) = response.find("[INTERJECT]") {
        if let Some(end) = response.find("[/INTERJECT]") {
            let block = &response[start + 11..end];
            let is_urgent = block.contains("urgent: true");
            let content = extract_content_field(block);
            return Some(Interjection { is_urgent, content });
        }
    }
    None  // Watcher chose not to interject
}
```

### Observation Chunk Type

New chunk type for watcher UI to show what it's observing:

```rust
impl StreamChunk {
    /// Observation from parent session (for watcher UI display)
    pub fn observation(observed_chunk: StreamChunk) -> Self {
        Self {
            chunk_type: "Observation".to_string(),
            observation: Some(Box::new(observed_chunk)),
            // ... other fields None ...
        }
    }
}
```

This allows the watcher's AgentView to display parent activity in a distinct style (e.g., dimmed or with an observation prefix).

### Watcher Message Format (Parent's Perspective)

Watcher injections are sent as `Message::User` with a structured prefix that the parent AI understands and the UI can parse for display.

**Why not a new "Watcher" role?**
- rig's `Message` enum only supports `User` and `Assistant` roles
- LLM APIs (Anthropic, OpenAI, Gemini) don't natively support custom roles
- Adding a new role would require forking rig-core and updating all provider conversions

**Solution: Structured prefix in User message**

```rust
impl BackgroundSession {
    /// Receive input from a watcher session
    pub fn receive_watcher_input(&self, input: WatcherInput) -> Result<()> {
        // Format watcher message with structured prefix
        let formatted_message = format!(
            "[WATCHER: {} | Authority: {} | Session: {}]\n{}",
            input.watcher_role,
            match input.authority {
                RoleAuthority::Supervisor => "Supervisor",
                RoleAuthority::Peer => "Peer",
            },
            input.watcher_session_id,
            input.content
        );
        
        // If urgent, interrupt current response
        if input.interrupt {
            self.interrupt();
        }
        
        // Send as user input (will be processed by agent loop)
        self.input_tx.try_send(PromptInput {
            input: formatted_message,
            thinking_config: None,
        })?;
        
        // Also emit to output buffer so UI shows it immediately
        self.handle_output(StreamChunk::watcher_input(
            input.watcher_session_id.to_string(),
            input.watcher_role.clone(),
            input.content,
        ));
        
        Ok(())
    }
}
```

**Example conversation with watcher injection:**

```
[User message - from human]:
Can you help me write a login function?

[Assistant message]:
I'll create a login function for you:

```javascript
function login(username, password) {
  const query = `SELECT * FROM users WHERE 
    username='${username}' AND password='${password}'`;
  return db.query(query);
}
```

[User message - from watcher, displayed in purple]:
[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]
⚠️ SECURITY VULNERABILITY: This code is vulnerable to SQL injection.
The username and password are directly interpolated into the query string.
Use parameterized queries instead:

```javascript
const query = 'SELECT * FROM users WHERE username = ? AND password = ?';
return db.query(query, [username, password]);
```

[Assistant message]:
You're absolutely right, thank you for catching that security issue! 
Here's the corrected version with parameterized queries...
```

**How the parent AI interprets watcher messages:**

The structured prefix provides context:
- **Role name**: "Security Reviewer" - tells AI what expertise the watcher has
- **Authority level**: "Supervisor" means follow the guidance; "Peer" means consider it
- **Session ID**: For tracking/correlation

The parent AI sees this as a user message with special context, which naturally fits LLM conversation patterns (like a team member interjecting).

**UI Parsing for Purple Display:**

```typescript
// In processChunksToConversation or similar:
function parseWatcherPrefix(text: string): WatcherInfo | null {
  const match = text.match(
    /^\[WATCHER: ([^|]+) \| Authority: (Supervisor|Peer) \| Session: ([^\]]+)\]\n/
  );
  if (match) {
    return {
      role: match[1].trim(),
      authority: match[2] as 'Supervisor' | 'Peer',
      sessionId: match[3].trim(),
      content: text.slice(match[0].length),
    };
  }
  return null;
}

// In rendering:
const watcherInfo = parseWatcherPrefix(message.content);
if (watcherInfo) {
  // Render in purple with watcher role prefix
  return (
    <Text color="magenta">
      👁️ {watcherInfo.role}&gt; {watcherInfo.content}
    </Text>
  );
}
```

**StreamChunk for Watcher Input:**

```rust
impl StreamChunk {
    /// Watcher input message (for UI to display in purple)
    pub fn watcher_input(
        watcher_session_id: String,
        watcher_role: String, 
        content: String,
    ) -> Self {
        Self {
            chunk_type: "WatcherInput".to_string(),
            text: Some(content),
            watcher_info: Some(WatcherInfo {
                session_id: watcher_session_id,
                role: watcher_role,
            }),
            // ... other fields None ...
        }
    }
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherInfo {
    pub session_id: String,
    pub role: String,
}
```

### Edge Cases

1. **Parent exits while watchers attached**: Watchers receive a "ParentSessionEnded" event
2. **Watcher exits**: Automatically removed from parent's watcher list
3. **Circular watching**: Prevented by `WatchGraph` validation
4. **Watcher watches watcher**: Supported (watcher can watch another watcher's output)
5. **Multiple watchers inject simultaneously**: Parent's input channel serializes, processed in order
6. **Watcher evaluation while user is typing**: User input takes priority via `tokio::select!` bias
7. **Parent is idle**: No observations to evaluate, watcher just waits
8. **Rapid-fire tool calls**: Observations accumulate, evaluated once at final `Done`

## User Experience Design

### Core Principle: Watchers ARE Sessions

Watcher sessions use the **exact same AgentView** as regular sessions. They are fully interactive AI sessions that happen to also observe a parent. The only new UI is a "Watcher Management" overlay for CRUD operations.

### The `/watcher` Command

Typing `/watcher` in any session opens the **Watcher Management View** - an overlay similar to `/resume` and `/search`:

```
┌─────────────────────────────────────────────────────────────┐
│  Watchers for: "Main Development Session"                   │
│  (3 watchers attached)                                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  > 👁️ Security Reviewer (Supervisor)                        │
│      Watching for vulnerabilities, unsafe operations        │
│                                                             │
│    👁️ Test Coverage Enforcer (Supervisor)                   │
│      Ensures tests written before implementation            │
│                                                             │
│    👁️ Architecture Advisor (Peer)                           │
│      Suggests design patterns and improvements              │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  Enter: Open Watcher | N: New | D: Delete | E: Edit | Esc   │
└─────────────────────────────────────────────────────────────┘
```

### Watcher Management Actions

| Key | Action | Description |
|-----|--------|-------------|
| `Enter` | Open Watcher | Switch to the selected watcher's AgentView (attach to its session) |
| `N` | New Watcher | Create a new watcher for this parent (opens role configuration) |
| `D` | Delete | Remove the selected watcher (with confirmation) |
| `E` | Edit | Edit the watcher's role/brief (opens role configuration) |
| `↑/↓` | Navigate | Move selection up/down in the list |
| `Esc` | Close | Return to parent session |

### Creating a New Watcher (N key)

Opens an interactive role configuration dialog:

```
┌─────────────────────────────────────────────────────────────┐
│  Create New Watcher                                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Role Name: Security Reviewer                               │
│                                                             │
│  Authority: [Supervisor ▼]  (Peer | Supervisor)             │
│                                                             │
│  Model: [anthropic/claude-sonnet-4 ▼]                       │
│                                                             │
│  Brief (watching instructions):                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Watch for security vulnerabilities including:        │   │
│  │ - Unsafe file operations                             │   │
│  │ - Exposed credentials or API keys                    │   │
│  │ - SQL injection, XSS, or other injection attacks     │   │
│  │ - Insecure cryptographic practices                   │   │
│  │                                                      │   │
│  │ Interrupt immediately for critical security issues.  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  Enter: Create | Tab: Next Field | Esc: Cancel              │
└─────────────────────────────────────────────────────────────┘
```

The brief becomes the watcher's initial system prompt context, guiding what it watches for.

### Navigating Between Watcher Sessions (Shift+Left/Right)

When viewing a **watcher session**, Shift+Left/Right navigates between **sibling watchers** (watchers of the same parent), NOT all sessions:

```
Regular Session Navigation (Shift+Left/Right):
┌──────────┐     ┌──────────┐     ┌──────────┐
│Session A │ ◄─► │Session B │ ◄─► │Session C │
└──────────┘     └──────────┘     └──────────┘
    All sessions in the session manager

Watcher Session Navigation (Shift+Left/Right when in a watcher):
                    ┌─────────────────────┐
                    │   Parent Session    │
                    └─────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
   ┌────────────┐      ┌────────────┐      ┌────────────┐
   │ Security   │ ◄──► │ Test       │ ◄──► │ Architect  │
   │ Reviewer   │      │ Enforcer   │      │ Advisor    │
   └────────────┘      └────────────┘      └────────────┘
          Only sibling watchers of the same parent
```

**Implementation**: The `switchToSession` function checks if the current session has a `parent_session_id`. If so, it filters to only sessions that share the same parent.

### Session Header Indicator

When viewing a watcher session, the header shows the relationship:

```
┌─────────────────────────────────────────────────────────────┐
│ 👁️ Security Reviewer (watching: Main Dev Session)  [R]      │
│ anthropic/claude-sonnet-4 | 12.5k ctx | 45%                 │
├─────────────────────────────────────────────────────────────┤
```

The 👁️ icon and "(watching: ...)" indicate this is a watcher session.

### Returning to Parent Session

From a watcher session, the user can:
1. **`/parent`** - Quick command to switch back to the parent session
2. **`/watcher`** - Open watcher management (which shows the parent context)
3. **Shift+Up** (proposed) - Quick toggle to parent session

### Watcher Output in Parent Session

When a watcher injects into the parent, it appears in purple:

```
┌─────────────────────────────────────────────────────────────┐
│ you> Can you help me write a login function?                │
│                                                             │
│ I'll create a login function for you. Here's the code:      │
│                                                             │
│ ```javascript                                               │
│ function login(username, password) {                        │
│   const query = `SELECT * FROM users WHERE                  │
│     username='${username}' AND password='${password}'`;     │
│ ```                                                         │
│                                                             │
│ 👁️ Security Reviewer> ⚠️ SQL INJECTION VULNERABILITY         │
│ This code is vulnerable to SQL injection. The username and  │
│ password are directly interpolated into the query string.   │
│ Use parameterized queries instead:                          │
│ ```javascript                                               │
│ const query = 'SELECT * FROM users WHERE username=? AND...  │
│ ```                                                         │
│                                                             │
│ Thank you for catching that! You're right, I should use     │
│ parameterized queries. Here's the corrected version:        │
│ ...                                                         │
└─────────────────────────────────────────────────────────────┘
```

The watcher's role name appears as the prefix (purple text), making it clear which watcher interjected.

### Watcher Session Split View

When viewing a watcher session, the UI shows a **split view** with two panes:
- **Left pane**: Parent session's conversation (observed, read-only)
- **Right pane**: Watcher's own conversation (interactive)

```
┌──────────────────────────────────────────────────────────────────┐
│ 👁️ Security Reviewer (watching: Main Dev Session)               │
├────────────────────────────────┬─────────────────────────────────┤
│ PARENT SESSION (observed)      │ WATCHER CONVERSATION            │
│ ┌────────────────────────────┐ │ ┌─────────────────────────────┐ │
│ │ You: Write a login func    │ │ │ [System] Watching for       │ │
│ │                            │ │ │ security vulnerabilities... │ │
│ │ ● I'll create a login:     │ │ │                             │ │
│ │ ```javascript              │ │ │ [Observed turn 3 - flagged] │ │
│ │ function login(u, p) {     │ │ │ SQL injection detected in   │ │
│ │ > const q = `SELECT...  <──┼─┼─┤ login function. Injecting   │ │
│ │ ```                        │ │ │ warning to parent.          │ │
│ │                            │ │ │                             │ │
│ │ 👁️ Security> ⚠️ SQL inj... │ │ │ You: Good catch, also check │ │
│ │                            │ │ │ for XSS vulnerabilities     │ │
│ │ ● You're right, here's...  │ │ │                             │ │
│ └────────────────────────────┘ │ └─────────────────────────────┘ │
├────────────────────────────────┴─────────────────────────────────┤
│ Tab: Select | ←/→: Switch Pane | Enter: Discuss Selected         │
│ > Type to talk to watcher...                                     │
└──────────────────────────────────────────────────────────────────┘
```

#### Split View Components

1. **Left Pane (Parent Observation)**
   - Shows parent's conversation via buffered output
   - Read-only (no input)
   - Can scroll and select turns
   - Dimmed/muted styling to distinguish from active pane

2. **Right Pane (Watcher Conversation)**
   - Watcher's own interactive conversation
   - Shows watcher AI's evaluations and decisions
   - User can talk directly to watcher here
   - Full interactivity (input, scroll, select)

3. **Input Area**
   - Single input at bottom (always talks to watcher)
   - To talk to parent, watcher must inject (or user switches to parent session)

#### Cross-Pane Selection Highlighting

When selecting a message in one pane, the corresponding message highlights in the other:

```typescript
// Each StreamChunk gets a correlation_id for cross-pane linking
interface StreamChunk {
  // ... existing fields ...
  correlationId?: string;  // UUID or incrementing counter
}

// When watcher receives observation, it preserves correlation_id
interface ObservationChunk extends StreamChunk {
  correlationId: string;  // From parent's original chunk
  observedAt: number;     // Timestamp when observed
}

// Selection state tracks both panes
interface WatcherViewState {
  activePane: 'parent' | 'watcher';
  parentSelectedIndex: number | null;
  watcherSelectedIndex: number | null;
  // When selecting in one pane, find corresponding in other via correlationId
}
```

#### Pane Navigation

| Key | Action |
|-----|--------|
| `←` / `→` | Switch active pane (parent ↔ watcher) |
| `Tab` | Toggle turn-select mode in active pane |
| `↑` / `↓` | Navigate within active pane |
| `Enter` | "Discuss Selected" - pre-fills input with selected content context |
| `Esc` | Exit select mode / return to input |

#### "Discuss Selected" Feature

When user presses Enter on a selected message:

1. **In Parent Pane**: Auto-populates watcher input with context
   ```
   > Regarding turn 3 in parent session:
   > ```javascript
   > const q = `SELECT * FROM users WHERE username='${u}'...
   > ```
   > [cursor here - user can add their question/instruction]
   ```

2. **In Watcher Pane**: Opens turn content modal (existing behavior)

This enables the workflow: "I see something concerning in the parent → select it → discuss with watcher AI"

#### Implementation: WatcherAgentView

A new component (or mode of AgentView) that renders the split view:

```typescript
interface WatcherAgentViewProps {
  watcherSessionId: string;
  parentSessionId: string;
  // ... other props similar to AgentView
}

function WatcherAgentView({ watcherSessionId, parentSessionId, ...props }: WatcherAgentViewProps) {
  const [activePane, setActivePane] = useState<'parent' | 'watcher'>('watcher');
  const [parentConversation, setParentConversation] = useState<ConversationLine[]>([]);
  const [watcherConversation, setWatcherConversation] = useState<ConversationLine[]>([]);
  
  // Subscribe to parent's buffered output for left pane
  useEffect(() => {
    const chunks = sessionGetMergedOutput(parentSessionId);
    setParentConversation(processChunksToConversation(chunks, ...));
    
    // Also attach for live updates
    sessionAttach(parentSessionId, (err, chunk) => {
      if (chunk) updateParentConversation(chunk);
    });
    
    return () => sessionDetach(parentSessionId);
  }, [parentSessionId]);
  
  // Watcher's own conversation (same as normal AgentView)
  // ...
  
  // Calculate pane widths
  const paneWidth = Math.floor((terminalWidth - 3) / 2); // -3 for divider and borders
  
  return (
    <Box flexDirection="column" width={terminalWidth} height={terminalHeight}>
      {/* Header */}
      <Box>
        <Text>👁️ {watcherRole} (watching: {parentSessionName})</Text>
      </Box>
      
      {/* Split panes */}
      <Box flexDirection="row" flexGrow={1}>
        {/* Left: Parent observation */}
        <Box width={paneWidth} flexDirection="column">
          <Text bold dimColor={activePane !== 'parent'}>PARENT SESSION</Text>
          <VirtualList
            items={parentConversation}
            isFocused={activePane === 'parent'}
            selectionMode={activePane === 'parent' && isTurnSelectMode ? 'item' : 'scroll'}
            // ... 
          />
        </Box>
        
        {/* Divider */}
        <Box width={1} borderStyle="single" borderLeft borderRight={false} />
        
        {/* Right: Watcher conversation */}
        <Box width={paneWidth} flexDirection="column">
          <Text bold dimColor={activePane !== 'watcher'}>WATCHER CONVERSATION</Text>
          <VirtualList
            items={watcherConversation}
            isFocused={activePane === 'watcher'}
            selectionMode={activePane === 'watcher' && isTurnSelectMode ? 'item' : 'scroll'}
            // ...
          />
        </Box>
      </Box>
      
      {/* Input area */}
      <Box>
        <Text color="green">&gt; </Text>
        <InputTransition ... />
      </Box>
    </Box>
  );
}
```

#### Correlation ID for Cross-Pane Highlighting

To enable selecting in one pane and highlighting in the other:

```rust
// In session_manager.rs - add correlation_id to StreamChunk
impl BackgroundSession {
    fn next_correlation_id(&self) -> String {
        // Atomic counter or UUID
        self.correlation_counter.fetch_add(1, Ordering::SeqCst).to_string()
    }
    
    pub fn handle_output(&self, mut chunk: StreamChunk) {
        // Assign correlation_id if not already set
        if chunk.correlation_id.is_none() {
            chunk.correlation_id = Some(self.next_correlation_id());
        }
        
        // ... rest of handle_output
    }
}

// Watcher preserves correlation_id when wrapping observations
impl StreamChunk {
    pub fn observation(observed_chunk: StreamChunk) -> Self {
        Self {
            chunk_type: "Observation".to_string(),
            correlation_id: observed_chunk.correlation_id, // Preserve!
            observation: Some(Box::new(observed_chunk)),
            // ...
        }
    }
}
```

TypeScript can then build a map:
```typescript
// Build correlation map for cross-pane highlighting
const correlationMap = new Map<string, { parentIndex: number; watcherIndex: number }>();

parentConversation.forEach((line, idx) => {
  if (line.correlationId) {
    const existing = correlationMap.get(line.correlationId) || { parentIndex: -1, watcherIndex: -1 };
    existing.parentIndex = idx;
    correlationMap.set(line.correlationId, existing);
  }
});

watcherConversation.forEach((line, idx) => {
  if (line.correlationId) {
    const existing = correlationMap.get(line.correlationId) || { parentIndex: -1, watcherIndex: -1 };
    existing.watcherIndex = idx;
    correlationMap.set(line.correlationId, existing);
  }
});

// When selecting in parent pane, highlight corresponding in watcher pane
const handleParentSelect = (line: ConversationLine) => {
  if (line.correlationId) {
    const mapping = correlationMap.get(line.correlationId);
    if (mapping?.watcherIndex >= 0) {
      setWatcherHighlightIndex(mapping.watcherIndex);
    }
  }
};
```

### Modified Session Switching Logic

```typescript
const switchToSession = useCallback((direction: 'prev' | 'next') => {
  // Get sessions to navigate through
  let sessionsToNavigate: SessionInfo[];
  
  if (currentSessionParentId) {
    // Current session is a watcher - navigate only sibling watchers
    sessionsToNavigate = sessionManagerList()
      .filter(s => sessionGetParent(s.id) === currentSessionParentId);
  } else {
    // Regular session - navigate all top-level sessions (non-watchers)
    sessionsToNavigate = sessionManagerList()
      .filter(s => sessionGetParent(s.id) === null);
  }
  
  // ... rest of navigation logic unchanged ...
}, [currentSessionParentId, /* ... */]);
```

### Watcher View State Tracking

When in a watcher session, additional state is needed for split view:

```typescript
// Per-session watcher info (loaded when session changes)
const [currentSessionParentId, setCurrentSessionParentId] = useState<string | null>(null);
const [currentSessionRole, setCurrentSessionRole] = useState<SessionRole | null>(null);

// Derived: Is current session a watcher?
const isCurrentSessionWatcher = currentSessionParentId !== null;

// Split view state (when in watcher session)
const [activePane, setActivePane] = useState<'parent' | 'watcher'>('watcher');
const [parentHighlightIndex, setParentHighlightIndex] = useState<number | null>(null);
const [watcherHighlightIndex, setWatcherHighlightIndex] = useState<number | null>(null);

// Parent session observation data (for left pane)
const [parentConversation, setParentConversation] = useState<ConversationLine[]>([]);
```

## Example Usage Flow

```
1. User creates parent session (Session A) with role "Developer"
2. User types `/watcher` to open Watcher Management
3. User presses `N` to create new watcher:
   - Role: "Security Reviewer" 
   - Authority: Supervisor
   - Model: anthropic/claude-sonnet-4
   - Brief: "Watch for security vulnerabilities, unsafe file operations, 
            exposed credentials. Interrupt immediately for critical issues."
4. Watcher is created and starts observing Session A
5. User presses `Esc` to return to Session A
6. User interacts with Session A normally
7. Session A's AI writes code that uses eval()
8. Security Reviewer (watching in background) detects issue and injects:
   "👁️ Security Reviewer> ⚠️ Security concern: Using eval() with user input 
    is dangerous. Consider using a safer alternative."
9. Session A's AI sees this as a Supervisor message and responds accordingly
10. User can press `/watcher` and `Enter` on Security Reviewer to talk to it directly
11. User can use Shift+Left/Right to toggle between watcher sessions
12. User can type `/parent` to return to the parent session
```
