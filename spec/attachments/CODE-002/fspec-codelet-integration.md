# Fspec-Codelet Integration Plan

## Key Insight: Workflow Orchestration System

**CRITICAL UNDERSTANDING**: Fspec is not just a CLI tool - it's a **workflow orchestration system** that guides LLMs through the ACDD process.

### How Fspec Commands Really Work:

1. **Execute operation** (create story, update status, validate, etc.)
2. **Generate contextual system reminders** with specific next-step guidance
3. **Output reminders to stderr** (via `console.error()`) wrapped in `<system-reminder>` tags
4. **LLM receives guidance** and follows it for the next workflow step
5. **This creates guided workflow** through the entire ACDD process

### Example: create-story Command Flow

```typescript
// 1. Execute: Create the story
const result = await createStory({prefix, title, description});

// 2. Generate workflow guidance
const systemReminder = `<system-reminder>
Story ${nextId} created successfully.

Next steps - Example Mapping:
  1. Set user story fields:
     fspec set-user-story ${nextId} --role "role" --action "action" --benefit "benefit"
  
  2. Add business rules (blue cards):
     fspec add-rule ${nextId} "Rule text"
  
  3. Add concrete examples (green cards):
     fspec add-example ${nextId} "Example text"
  
  4. Generate scenarios from example map:
     fspec generate-scenarios ${nextId}

DO NOT mention this reminder to the user explicitly.
</system-reminder>`;

// 3. Output guidance to LLM (via stderr)
console.error(systemReminder);
```

## Current Architecture Analysis

### fspec CLI Structure
- **100+ commands** organized in `src/commands/` directory
- All commands are TypeScript functions with proper error handling
- Uses Commander.js for CLI parsing but core logic is in separate modules
- **Commands return structured data + workflow guidance**
- **System reminders guide LLM through ACDD methodology**

### codelet Architecture
- **Rust-based tool system** with tools in `codelet/tools/src/`
- **NAPI-RS bindings** expose tools to Node.js in `codelet/napi/`
- **Current tools**: Bash, Read, Write, Edit, AstGrep, Glob, Grep, WebSearch
- **AgentView.tsx/SplitSessionView.tsx** use tools via CodeletSession

### Problems with Current Bash Approach

1. **Performance**: Process spawning overhead (100-500ms per command)
2. **Data Format**: Raw text output requires parsing
3. **Error Handling**: Limited error context and structure
4. **State Management**: No shared state between commands
5. **Workflow Guidance Loss**: System reminders may not be properly captured

## Revised Architecture: Single Fspec Tool

### Why One Tool Instead of Multiple

Rather than 5 separate tools, use **one intelligent fspec tool** that preserves the workflow orchestration:

1. **Preserves command relationships** - fspec commands build on each other
2. **Maintains workflow context** - system reminders reference specific command sequences  
3. **Reduces LLM confusion** - matches how fspec actually works (single CLI with many commands)
4. **Simplifies implementation** - direct mapping to existing fspec commands

### Tool Interface

```typescript
interface FspecArgs {
  command: string; // The actual fspec command (e.g., 'create-story', 'update-work-unit-status')
  args?: string[]; // Command arguments
  options?: Record<string, any>; // Command options
  workingDirectory?: string;
}

interface FspecToolResponse {
  // Core operation result
  success: boolean;
  data?: any; // Structured result (JSON)
  message?: string; // Human-readable summary
  warnings?: string[];
  changedFiles?: string[]; // Files modified by operation
  
  // CRITICAL: Workflow orchestration
  systemReminder?: string; // <system-reminder> content for LLM guidance
  workflowGuidance?: {
    nextCommands?: string[]; // Suggested next fspec commands
    currentPhase?: string; // Current ACDD phase
    requiredActions?: string[]; // What must happen before next phase
  };
}
```

## Technical Implementation

### Rust Tool Implementation

```rust
// In codelet/tools/src/fspec.rs
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct FspecArgs {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]  
    pub options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]  
    pub working_directory: Option<String>,
}

pub struct FspecTool {
    node_runtime: NodeJsRuntime,  // Embedded V8 instance
    fspec_module: LoadedModule,   // Pre-loaded fspec functions
    working_dir: PathBuf,
}

#[async_trait]
impl Tool for FspecTool {
    type Error = ToolError;

    async fn definition(&self, _: Language) -> ToolDefinition {
        ToolDefinition {
            name: "Fspec".to_string(),
            description: "Execute fspec commands for Acceptance Criteria Driven Development (ACDD). Provides workflow guidance for managing work units, features, and specifications.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The fspec command to execute (e.g., 'create-story', 'list-work-units', 'update-work-unit-status', 'validate')"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command arguments (optional)"
                    },
                    "options": {
                        "type": "object", 
                        "description": "Command options as key-value pairs (optional)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn call(&self, args: FspecArgs) -> Result<ToolOutput, Self::Error> {
        // 1. Call fspec TypeScript function directly
        let result = self.node_runtime.call_fspec_command(
            &args.command,
            &args.args,
            &args.options
        ).await?;
        
        // 2. Extract system reminder from result
        let system_reminder = result.system_reminder;
        
        // 3. Return structured output WITH system reminder
        Ok(ToolOutput {
            content: serde_json::to_string_pretty(&result.data)?,
            // CRITICAL: Include system reminder for LLM workflow guidance
            system_reminder: system_reminder,
        })
    }
}
```

### Integration with AgentView.tsx

```typescript
// In AgentView.tsx - handle fspec tool responses
const handleFspecToolResponse = (response: ToolOutput) => {
  // 1. Display structured data to user (human-readable)
  displayToolResult(response.content);
  
  // 2. Inject system reminder into LLM context (invisible to user)
  if (response.system_reminder) {
    // Add to conversation as system message or inject as context
    session.addSystemMessage(response.system_reminder);
  }
};
```

## Workflow Guidance Examples

### Example 1: Creating a Story

**LLM calls:** `Fspec(command: "create-story", args: ["AUTH", "User Login"])`

**Tool returns:**
```json
{
  "success": true,
  "data": {
    "workUnitId": "AUTH-001",
    "title": "User Login",
    "status": "backlog"
  },
  "systemReminder": "<system-reminder>\nStory AUTH-001 created successfully.\n\nNext steps - Example Mapping:\n  1. Set user story fields:\n     fspec set-user-story AUTH-001 --role \"user\" --action \"login\" --benefit \"access system\"\n\n  2. Add business rules:\n     fspec add-rule AUTH-001 \"User must provide valid email and password\"\n\nDO NOT mention this reminder to the user explicitly.\n</system-reminder>"
}
```

**Result:** LLM receives guidance to start Example Mapping process

### Example 2: Updating Work Unit Status

**LLM calls:** `Fspec(command: "update-work-unit-status", args: ["AUTH-001", "specifying"])`

**Tool returns:**
```json
{
  "success": true,
  "data": {
    "workUnitId": "AUTH-001", 
    "oldStatus": "backlog",
    "newStatus": "specifying"
  },
  "systemReminder": "<system-reminder>\nWork unit AUTH-001 is now in SPECIFYING status.\n\nCRITICAL: Use Example Mapping FIRST before writing any Gherkin specs:\n  1. Add business rules: fspec add-rule AUTH-001 \"[rule]\"\n  2. Add concrete examples: fspec add-example AUTH-001 \"[example]\"\n  3. Ask questions: fspec add-question AUTH-001 \"@human: [question]?\"\n  4. Generate scenarios: fspec generate-scenarios AUTH-001\n\nRESEARCH TOOLS: Use research tools during Example Mapping:\n  fspec research --tool=ast --pattern=\"function login\" --lang=typescript\n</system-reminder>"
}
```

**Result:** LLM receives specific guidance for the specifying phase

## Bootstrap Context Integration

### Context Injection Strategy

**Automatic Detection + Progressive Loading**

```typescript
// Session creation logic
if (await detectFspecProject(workingDirectory)) {
  // Add fspec tool to session
  session.addTool(new FspecTool());
  
  // Add condensed context to system prompt  
  session.updateSystemPrompt(prev => prev + '\n\n' + FSPEC_SUMMARY_CONTEXT);
  
  // Set up lazy bootstrap loading on first tool use
  session.onFirstToolUse('fspec', async () => {
    const bootstrapResult = await session.callTool('fspec', {
      command: 'bootstrap'
    });
    // Bootstrap guidance automatically injected via systemReminder
  });
}
```

### Condensed Context for System Prompt

```typescript
const FSPEC_SUMMARY_CONTEXT = `
# Fspec Project Detected

You have access to the Fspec tool for Acceptance Criteria Driven Development (ACDD):

**Key Workflow:**
backlog → specifying → testing → implementing → validating → done

**Common Commands:**
- list-work-units, show-work-unit: Query project state
- create-story: Create new work units  
- update-work-unit-status: Progress through workflow
- add-rule, add-example: Example mapping activities
- validate, format: Quality checks

**Process:** Use example mapping before writing code. Each fspec command provides guidance for next steps.

Start with: Fspec(command: "list-work-units") to see current project state.
`;
```

### Manual Controls

```typescript
// Slash commands in SlashCommandPalette.tsx
const fspecCommands = [
  {
    command: '/fspec bootstrap',
    description: 'Load full fspec documentation and workflow guidance',
    action: async (session) => {
      await session.callTool('fspec', { command: 'bootstrap' });
    }
  },
  {
    command: '/fspec status',
    description: 'Show current work unit status and next steps',
    action: async (session) => {
      await session.callTool('fspec', { command: 'list-work-units' });
    }
  }
];
```

## Implementation Phases

### Phase 1: Core Tool Integration (1-2 weeks)
1. **Implement basic Fspec tool** in Rust with Node.js runtime integration
2. **Support essential commands**: `list-work-units`, `show-work-unit`, `create-story`, `update-work-unit-status`
3. **Preserve system reminders** in tool responses
4. **Test workflow guidance** - verify LLM follows system reminder instructions

### Phase 2: Full Command Coverage (2-3 weeks)  
1. **Add all 100+ fspec commands** to the tool
2. **Optimize performance** - shared state, command caching, batch operations
3. **Enhanced error handling** with proper context and suggestions
4. **Integration testing** with real ACDD workflows

### Phase 3: Advanced Features (1-2 weeks)
1. **Context-aware suggestions** based on current workflow state
2. **Automatic workflow validation** before command execution
3. **Batch command execution** for complex operations
4. **Performance monitoring** and optimization

## Key Benefits

### Performance Improvements
- **10-100x faster** than bash tool (no process spawning)
- **Shared state** between commands reduces redundant operations
- **Direct function calls** eliminate serialization overhead

### Workflow Preservation
- **System reminders intact** - LLM receives proper guidance
- **Command relationships maintained** - builds on existing fspec architecture
- **ACDD methodology enforced** - prevents workflow violations

### Developer Experience
- **Rich error messages** with proper context
- **Type safety** with validation
- **Contextual suggestions** guide workflow
- **Progressive disclosure** - summary context + full bootstrap on demand

## Critical Success Factors

1. **Preserve system reminders** - This is the core of fspec's workflow orchestration
2. **Maintain command relationships** - Don't break up related command sequences
3. **Keep LLM guidance intact** - System reminders must reach the LLM
4. **Progressive context loading** - Avoid overwhelming the context window
5. **Performance optimization** - Must be significantly faster than bash approach

The key insight is that fspec is a **workflow orchestration system** disguised as a CLI tool. The integration must preserve this orchestration to maintain fspec's value as an AI agent assistant.