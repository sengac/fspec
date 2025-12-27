# Dynamic Thinking Level Detection - Implementation Plan

## Overview

This story integrates the `ThinkingConfigFacade` (TOOL-009) with keyword-based detection to dynamically set thinking/reasoning levels based on prompt content. Users can control thinking depth using natural language keywords like "ultrathink" or disable thinking with "quickly".

## Dependencies

- **TOOL-009**: ThinkingConfigFacade for Provider-Specific Reasoning Configuration (DONE)

## Research: Claude Code's Approach

Claude Code intercepts thinking keywords in the JavaScript layer before sending to the API:

| Level | Keywords | Token Budget |
|-------|----------|--------------|
| Low | `think` | ~4,000 tokens |
| Medium | `think hard`, `think deeply`, `think more`, `megathink` | ~10,000 tokens |
| High | `think harder`, `think very hard`, `ultrathink` | ~32,000 tokens |

**Sources:**
- [Claude Code Thinking Levels Guide](https://www.vibesparking.com/en/blog/ai/claude-code/2025-07-28-claude-code-thinking-modes-guide/)
- [Think, Megathink, Ultrathink Decoded](https://smartnested.com/think-megathink-ultrathink-claude-codes-power-keywords-decoded/)

## Design Decisions

### 1. Keyword Parsing (Not Toggle)

**Why keywords over toggle:**
- Self-documenting prompts
- No hidden state conflicts
- Portable across sessions
- Familiar to Claude Code users
- Zero UI complexity

### 2. Disable Keywords Override

Disable keywords have **highest priority** and always force `ThinkingLevel::Off`:
- `quickly`, `brief`, `briefly`, `fast`
- `nothink`, `no thinking`, `don't think hard`, `don't overthink`

### 3. Strict Pattern Matching

Only match command-like phrases, NOT conversational usage:

**TRIGGER (commands):**
```
"think about this bug"          → Low
"think through the problem"     → Low
"think carefully here"          → Low
"megathink"                     → Medium
"ultrathink"                    → High
```

**DO NOT TRIGGER (conversational):**
```
"I think we should..."          → Off (opinion)
"what do you think?"            → Off (question)
"don't think so"                → Off (negation)
"I was thinking about..."       → Off (past tense)
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  User types prompt in AgentModal                            │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  TypeScript: detectThinkingLevel(prompt)                    │
│  Location: src/utils/thinkingLevel.ts                       │
│  Returns: JsThinkingLevel enum                              │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  TypeScript: getThinkingConfig(provider, level)             │
│  Location: @sengac/codelet-napi (already exists)            │
│  Returns: JSON string with provider-specific config         │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  AgentModal: Shows thinking indicator in UI                 │
│  Location: src/tui/components/AgentModal.tsx                │
│  Display: "🧠 High" / "🧠 Medium" / "🧠 Low" / (none)        │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Rust: CodeletSession receives thinking config              │
│  Location: codelet/napi/src/session.rs                      │
│  Merges config into request additional_params               │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Provider receives correct thinkingConfig                   │
│  Gemini 3: { thinkingLevel: "high" }                        │
│  Gemini 2.5: { thinkingBudget: 8192 }                       │
│  Claude: { thinking: { budget_tokens: 32000 } }             │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Details

### 1. TypeScript: Thinking Level Detector

**File:** `src/utils/thinkingLevel.ts`

```typescript
import { JsThinkingLevel } from '@sengac/codelet-napi';

// Keywords organized by priority (check disable first)
const DISABLE_KEYWORDS = [
  'quickly', 'brief', 'briefly', 'fast',
  'nothink', 'no thinking', "don't think hard", "don't overthink"
];

const HIGH_PATTERNS = [
  /\bultrathink\b/,
  /\bthink\s+(harder|intensely|very\s+hard|super\s+hard|really\s+hard|longer)\b/
];

const MEDIUM_PATTERNS = [
  /\bmegathink\b/,
  /\bthink\s+(hard|deeply|more|a\s+lot)\b/
];

const LOW_PATTERNS = [
  /\bthink\s+(about|through|carefully)\b/,
  /^think\b/,                    // Starts with "think"
  /[:.]\s*think\b/               // After colon or period
];

// Negative patterns - DO NOT match these
const CONVERSATIONAL_PATTERNS = [
  /\bi\s+think\b/,               // "I think..."
  /\bwhat\s+do\s+you\s+think\b/, // "what do you think"
  /\bdon'?t\s+think\s+so\b/,     // "don't think so"
  /\bwas\s+thinking\b/,          // "was thinking"
  /\bthinking\s+about\b/         // "thinking about" (gerund)
];

export function detectThinkingLevel(prompt: string): JsThinkingLevel {
  const lower = prompt.toLowerCase();

  // 1. DISABLE keywords have highest priority
  if (DISABLE_KEYWORDS.some(kw => lower.includes(kw))) {
    return JsThinkingLevel.Off;
  }

  // 2. Skip if conversational usage
  if (CONVERSATIONAL_PATTERNS.some(p => p.test(lower))) {
    return JsThinkingLevel.Off;
  }

  // 3. Check HIGH level
  if (HIGH_PATTERNS.some(p => p.test(lower))) {
    return JsThinkingLevel.High;
  }

  // 4. Check MEDIUM level
  if (MEDIUM_PATTERNS.some(p => p.test(lower))) {
    return JsThinkingLevel.Medium;
  }

  // 5. Check LOW level
  if (LOW_PATTERNS.some(p => p.test(lower))) {
    return JsThinkingLevel.Low;
  }

  // 6. Default: Off
  return JsThinkingLevel.Off;
}

export function getThinkingLevelLabel(level: JsThinkingLevel): string | null {
  switch (level) {
    case JsThinkingLevel.High:
      return '🧠 High';
    case JsThinkingLevel.Medium:
      return '🧠 Medium';
    case JsThinkingLevel.Low:
      return '🧠 Low';
    default:
      return null;
  }
}
```

### 2. TypeScript: AgentModal Integration

**File:** `src/tui/components/AgentModal.tsx`

```typescript
import { detectThinkingLevel, getThinkingLevelLabel } from '../../utils/thinkingLevel';
import { getThinkingConfig, JsThinkingLevel } from '@sengac/codelet-napi';

// In component state
const [thinkingLevel, setThinkingLevel] = useState<JsThinkingLevel>(JsThinkingLevel.Off);

// When user types, detect thinking level for display
const handleInputChange = (value: string) => {
  setInputValue(value);
  setThinkingLevel(detectThinkingLevel(value));
};

// When submitting prompt
const handleSubmit = async () => {
  const level = detectThinkingLevel(inputValue);
  const thinkingConfig = level !== JsThinkingLevel.Off
    ? getThinkingConfig(currentProvider, level)
    : null;

  // Pass to session
  await session.prompt(inputValue, { thinkingConfig });
};

// In render - show indicator
{thinkingLevel !== JsThinkingLevel.Off && (
  <Text color="cyan">{getThinkingLevelLabel(thinkingLevel)}</Text>
)}
```

### 3. Rust: Session Modification

**File:** `codelet/napi/src/session.rs`

Add thinking config parameter to prompt method:

```rust
#[napi]
pub async fn prompt(
    &mut self,
    message: String,
    thinking_config: Option<String>,  // JSON string from getThinkingConfig()
) -> napi::Result<()> {
    // Parse and merge thinking config into additional_params
    if let Some(config_json) = thinking_config {
        let config: serde_json::Value = serde_json::from_str(&config_json)?;
        // Merge into request builder's additional_params
        self.merge_additional_params(config)?;
    }

    // ... rest of prompt handling
}
```

### 4. Rust: Request Building

The thinking config needs to be merged into `additional_params` which gets passed to the provider's `GenerationConfig`:

```rust
// In request building code
fn merge_additional_params(&mut self, thinking_config: Value) -> Result<()> {
    // For Gemini: merge into generationConfig.thinkingConfig
    // For Claude: merge into thinking field
    // The facade already generates the correct structure
}
```

## Keyword Reference

### Enable Keywords (by level)

| Level | Keywords |
|-------|----------|
| **High** | `ultrathink`, `think harder`, `think intensely`, `think longer`, `think really hard`, `think super hard`, `think very hard` |
| **Medium** | `megathink`, `think hard`, `think deeply`, `think more`, `think a lot` |
| **Low** | `think about [X]`, `think through [X]`, `think carefully`, `think` (at start of prompt) |

### Disable Keywords (highest priority)

| Keywords |
|----------|
| `quickly`, `brief`, `briefly`, `fast` |
| `nothink`, `no thinking`, `don't think hard`, `don't overthink` |

### Non-Triggering Patterns (conversational)

| Pattern | Example |
|---------|---------|
| `I think...` | "I think we should refactor this" |
| `what do you think` | "What do you think about this approach?" |
| `don't think so` | "I don't think so" |
| `was thinking` | "I was thinking about the design" |
| `thinking about` | "I've been thinking about this" |

## Testing Strategy

### Unit Tests (TypeScript)

```typescript
describe('detectThinkingLevel', () => {
  // High level
  test('ultrathink returns High', () => {
    expect(detectThinkingLevel('ultrathink about this')).toBe(JsThinkingLevel.High);
  });

  // Disable overrides
  test('quickly overrides ultrathink', () => {
    expect(detectThinkingLevel('ultrathink but quickly')).toBe(JsThinkingLevel.Off);
  });

  // Conversational does not trigger
  test('I think does not trigger', () => {
    expect(detectThinkingLevel('I think we should fix this')).toBe(JsThinkingLevel.Off);
  });
});
```

### Integration Tests

- Verify thinking config is correctly passed through session
- Verify provider receives correct format (Gemini vs Claude)
- Verify UI indicator updates on input change

## Files to Create/Modify

### New Files
- `src/utils/thinkingLevel.ts` - Keyword detection logic
- `src/utils/__tests__/thinkingLevel.test.ts` - Unit tests

### Modified Files
- `src/tui/components/AgentModal.tsx` - Detection integration, UI indicator
- `codelet/napi/src/session.rs` - Accept thinking config parameter
- `codelet/napi/index.d.ts` - Update TypeScript declarations if needed

## Acceptance Criteria

1. ✅ Typing "ultrathink" in prompt shows "🧠 High" indicator
2. ✅ Typing "megathink" shows "🧠 Medium" indicator
3. ✅ Typing "think about this" shows "🧠 Low" indicator
4. ✅ Typing "I think we should" shows NO indicator (conversational)
5. ✅ Typing "ultrathink but quickly" shows NO indicator (disable wins)
6. ✅ Provider receives correct thinkingConfig format
7. ✅ AI response uses extended thinking when configured
