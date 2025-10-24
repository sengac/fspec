# BUG-031: ULTRATHINK Multi-Agent Support Investigation

**Date**: 2025-10-24
**Investigator**: Claude (AI Agent)
**Work Unit**: BUG-031 - No support for ULTRATHINK in discovery-driven feedback loop
**Status**: Investigation Complete → Ready for Implementation

---

## Executive Summary

The `discover-foundation` command currently **hardcodes** the term "ULTRATHINK" in its system-reminder guidance, which is Claude-specific terminology for extended thinking/deep analysis. Other AI agents (Gemini, Cursor, Cline, Aider, etc.) don't understand this term, causing confusion during the discovery workflow.

**Good News**: The infrastructure to fix this already exists via INIT-008 (completed). The fix is straightforward - replace hardcoded ULTRATHINK with agent-aware messaging using existing `agentRuntimeConfig.ts` utilities.

**Recommendation**: Implement Option 1 (Quick Fix) - 2-3 hours effort, high ROI, unblocks multi-agent discovery.

---

## 🔍 Investigation Findings

### 1. Where ULTRATHINK Appears (12 Files Found)

#### **Source Code (3 files)**
1. **`/Users/rquast/projects/fspec/src/commands/discover-foundation.ts`** - Main implementation
   - **Lines 88-110**: Field-specific guidance generation
   - **Lines 422-429**: Initial draft creation system-reminder
   - Contains hardcoded "ULTRATHINK: Read ALL code, understand deeply"

2. **`/Users/rquast/projects/fspec/src/commands/discover-foundation-help.ts`** - Help documentation
   - References ULTRATHINK in command help text

3. **`/Users/rquast/projects/fspec/src/utils/slashCommandSections/bootstrapFoundation.ts`** - Slash command section
   - Documents ULTRATHINK usage in bootstrap process

#### **Tests (1 file)**
4. **`/Users/rquast/projects/fspec/src/commands/__tests__/draft-driven-discovery-feedback-loop.test.ts`**
   - Test validates ULTRATHINK appears in system-reminders
   - Will need updating to test both Claude and non-Claude agents

#### **Feature Files (2 files)**
5. **`/Users/rquast/projects/fspec/spec/features/implement-draft-driven-discovery-workflow-with-ai-chaining.feature`**
   - Lines 31-33, 48, 74, 79: Scenarios testing ULTRATHINK guidance
   - Defines expected behavior for ULTRATHINK-driven discovery

6. **`/Users/rquast/projects/fspec/spec/features/implement-draft-driven-discovery-workflow-with-ai-chaining.feature.coverage`**
   - Coverage tracking for ULTRATHINK scenarios

#### **Documentation (3 files)**
7. **`/Users/rquast/projects/fspec/spec/FOUNDATION.md`** - Auto-generated documentation
8. **`/Users/rquast/projects/fspec/spec/foundation.json`** - JSON data with Mermaid diagrams
9. **`/Users/rquast/projects/fspec/spec/work-units.json`** - Contains BUG-031 definition

#### **Attachments (2 files)**
10. **`/Users/rquast/projects/fspec/spec/attachments/GIT-003/git-001-critical-analysis.md`**
11. **`/Users/rquast/projects/fspec/spec/attachments/DISC-001/discovery-feedback-loop.mmd`**

#### **Configuration (1 file)**
12. **`/Users/rquast/projects/fspec/spec/features/fix-git-001-critical-bugs-and-logic-errors.feature`**

---

### 2. What ULTRATHINK Is Used For

**Purpose**: Instructs AI agents to perform deep codebase analysis during foundation discovery.

**Context**: Used in `discover-foundation` command's system-reminders to guide AI through field-by-field foundation completion.

**Example from discover-foundation.ts (Lines 88-110)**:
```typescript
function generateFieldReminder(fieldPath: string, fieldNum: number, totalFields: number) {
  const reminders = {
    'project.vision': `Field ${fieldNum}/${totalFields}: project.vision (elevator pitch)

ULTRATHINK: Read ALL code, understand the system deeply. What is the core PURPOSE?
Focus on WHY this exists, not HOW it works.

Ask human to confirm vision.

Run: fspec update-foundation projectVision "your vision"`,
    // ... other fields
  };
}
```

**Initial Draft Creation (Lines 422-429)**:
```typescript
const systemReminder = `Draft created. To complete foundation, you must ULTRATHINK the entire codebase.

Analyze EVERYTHING: code structure, entry points, user interactions, documentation.
Understand HOW it works, then determine WHY it exists and WHAT users can do.

I will guide you field-by-field.`;
```

**Key Usage Points**:
- ✅ **Deep analysis instruction**: "Read ALL code, understand deeply"
- ✅ **WHY/WHAT focus**: Guides toward purpose and capabilities, not implementation
- ✅ **Wrapped in system-reminders**: Visible to AI, invisible to users
- ❌ **Claude-specific**: Other agents don't understand "ULTRATHINK" terminology

---

### 3. The Problem: BUG-031

**Issue**: ULTRATHINK is Claude Code's extended thinking mode terminology. Other AI agents:
- **Gemini**: No equivalent concept
- **Cursor**: Uses inline context, not extended thinking
- **Cline**: Doesn't support meta-cognitive prompts
- **Aider**: CLI-focused, no extended thinking
- **Others**: 15+ agents don't recognize this term

**Impact**:
- Non-Claude agents see confusing "ULTRATHINK" instructions
- Discovery workflow guidance is less effective for 18 out of 19 supported agents
- Violates multi-agent support goal established in INIT-008

**From work-units.json**:
```json
{
  "id": "BUG-031",
  "title": "No support for ULTRATHINK in discovery-driven feedback loop",
  "description": "The discover-foundation command uses the term 'ULTRATHINK' in its guidance system-reminders to instruct AI agents to perform deep analysis. However, not all AI agents support extended thinking or use this terminology. Claude Code may understand 'ULTRATHINK', but Gemini, Cursor, and other agents do not. The discovery-driven feedback loop needs agent-specific language that adapts to the capabilities of the detected AI agent.",
  "dependsOn": ["INIT-008"]
}
```

---

### 4. Dependency Analysis: INIT-008 (Completed)

**Status**: ✅ **DONE** - All infrastructure already exists

#### **What INIT-008 Delivered**:

**1. Agent Detection System** (`src/utils/agentDetection.ts`):
```typescript
export function detectAgents(cwd: string): DetectedAgent[] {
  const detected: DetectedAgent[] = [];
  for (const agent of AGENT_REGISTRY) {
    for (const detectionPath of agent.detectionPaths) {
      const fullPath = join(cwd, detectionPath);
      if (existsSync(fullPath)) {
        detected.push({ agent, detectedPath: detectionPath });
        break;
      }
    }
  }
  return detected;
}
```
- Checks for agent-specific directories (`.claude/`, `.cursor/`, `.cline/`, etc.)
- Returns detected agents with metadata

**2. Agent Registry** (`src/utils/agentRegistry.ts`):
```typescript
interface AgentConfig {
  id: string;
  name: string;
  supportsSystemReminders: boolean;    // Claude-only
  supportsMetaCognition: boolean;      // Claude-only (extended thinking)
  category: 'ide' | 'cli' | 'extension';
  detectionPaths: string[];
  // ... more properties
}
```
- **19 agents supported**: Claude, Cursor, Cline, Aider, Windsurf, Copilot, Gemini, Qwen, and 11 more
- **Metadata flags**: `supportsMetaCognition` identifies agents that understand extended thinking

**3. Runtime Configuration** (`src/utils/agentRuntimeConfig.ts`):
```typescript
export function getAgentConfig(cwd: string): AgentConfig {
  // Priority 1: FSPEC_AGENT environment variable
  const envAgent = process.env.FSPEC_AGENT;
  if (envAgent) {
    const agent = getAgentById(envAgent);
    if (agent) return agent;
  }

  // Priority 2: spec/fspec-config.json file
  const configPath = join(cwd, 'spec', 'fspec-config.json');
  if (existsSync(configPath)) {
    const config = JSON.parse(readFileSync(configPath, 'utf-8'));
    const agent = getAgentById(config.agent);
    if (agent) return agent;
  }

  // Priority 3: Safe default (no system-reminders)
  return DEFAULT_AGENT;
}
```
- **3-tier priority**: Env var → Config file → Safe default
- Returns agent configuration for current runtime

**4. Output Formatting** (`src/utils/agentRuntimeConfig.ts`):
```typescript
export function formatAgentOutput(agent: AgentConfig, message: string): string {
  // Claude Code gets system-reminder tags
  if (agent.supportsSystemReminders) {
    return `<system-reminder>\n${message}\n</system-reminder>`;
  }

  // IDE agents (Cursor, Cline, Windsurf) get emoji warnings
  if (agent.category === 'ide' || agent.category === 'extension') {
    return `**⚠️ IMPORTANT:** ${message}`;
  }

  // CLI agents (Aider, Gemini) get plain warnings
  return `**IMPORTANT:** ${message}`;
}
```
- Different output formats per agent type
- Already handles system-reminder wrapping

**5. Agent-Specific Files**:
- `spec/CLAUDE.md`, `spec/CURSOR.md`, etc. - Generated per agent
- `.claude/commands/fspec.md`, `.cursor/commands/fspec.md` - Slash commands
- `spec/fspec-config.json` - Stores selected agent

#### **What This Means**:
✅ All infrastructure exists to detect which agent is running
✅ Helper functions available for agent-aware output
✅ Registry includes `supportsMetaCognition` flag (exactly what we need!)
✅ No new utilities needed - just use existing functions

---

### 5. Current Implementation Analysis

**File**: `src/commands/discover-foundation.ts`

#### **Hardcoded ULTRATHINK Locations**:

**Location 1: Initial Draft System-Reminder (Line ~422)**
```typescript
const systemReminder = `Draft created. To complete foundation, you must ULTRATHINK the entire codebase.

Analyze EVERYTHING: code structure, entry points, user interactions, documentation.
Understand HOW it works, then determine WHY it exists and WHAT users can do.

I will guide you field-by-field.`;
```

**Location 2: Field Guidance for project.vision (Line ~95)**
```typescript
'project.vision': `Field ${fieldNum}/${totalFields}: project.vision (elevator pitch)

ULTRATHINK: Read ALL code, understand the system deeply. What is the core PURPOSE?
Focus on WHY this exists, not HOW it works.

Ask human to confirm vision.

Run: fspec update-foundation projectVision "your vision"`,
```

#### **Current Behavior**:
- ❌ Sends "ULTRATHINK" to all agents
- ❌ No runtime agent detection
- ❌ No conditional messaging based on agent capabilities
- ❌ Tests only validate Claude behavior

---

## 💡 Recommended Solution

### **Option 1: Quick Fix (RECOMMENDED)**

**Leverage existing INIT-008 infrastructure for agent-aware messaging.**

#### **Implementation Steps**:

**Step 1: Import Agent Detection**
```typescript
// At top of src/commands/discover-foundation.ts
import { getAgentConfig } from '../utils/agentRuntimeConfig';
```

**Step 2: Detect Current Agent**
```typescript
// In generateFieldReminder() function
const agent = getAgentConfig(cwd);
```

**Step 3: Conditional Messaging**
```typescript
const thinkingPrompt = agent.supportsMetaCognition
  ? "ULTRATHINK: Read ALL code, understand the system deeply."
  : "Carefully analyze the entire codebase to understand its purpose.";

const thinkingInstruction = agent.supportsMetaCognition
  ? "you must ULTRATHINK the entire codebase"
  : "you must thoroughly analyze the entire codebase";
```

**Step 4: Update 2 Key Locations**
- Initial draft system-reminder (line ~422)
- Field guidance for `project.vision` (line ~95)

**Step 5: Update Tests**
```typescript
// Add test for non-Claude agents
describe('Scenario: Non-Claude agent guidance without ULTRATHINK', () => {
  it('should use generic analysis language', async () => {
    process.env.FSPEC_AGENT = 'cursor';
    const result = await discoverFoundation({ cwd });
    expect(result.systemReminder).not.toContain('ULTRATHINK');
    expect(result.systemReminder).toContain('Carefully analyze');
  });
});
```

#### **Effort Estimate**: 2-3 hours
- ✅ Simple string substitution pattern
- ✅ All utilities already exist
- ✅ Clear test cases
- ✅ Low risk, high ROI

---

### **Option 2: Comprehensive Refactor (Future Enhancement)**

**Create agent-specific guidance library with customizable discovery workflows.**

#### **Scope**:
- Agent-specific field prompts
- Different discovery strategies per agent type
- Customizable workflow templates
- Enhanced agent capability detection

#### **Effort Estimate**: 8+ hours
- ❌ Over-engineering for current need
- ❌ Adds complexity without clear ROI
- ⚠️ Better as future enhancement after Option 1 proves successful

---

## 🎯 Recommendation

**Implement Option 1 (Quick Fix) NOW** because:

| Factor | Assessment |
|--------|------------|
| **Infrastructure** | ✅ Already exists (INIT-008 complete) |
| **Effort** | ✅ Low (2-3 hours) |
| **Risk** | ✅ Low (simple substitution, existing patterns) |
| **ROI** | ✅ High (unblocks 18 out of 19 agents) |
| **Critical Path** | ✅ Yes (first-run onboarding command) |
| **Testing** | ✅ Clear test cases (Claude vs non-Claude) |

### **Why Option 1 is Best**:
1. **Quick win**: Unblocks multi-agent discovery immediately
2. **Low risk**: Uses proven patterns from INIT-008
3. **Maintainable**: Simple conditional logic, easy to understand
4. **Extensible**: Can enhance later with Option 2 if needed
5. **User impact**: Improves onboarding for 95% of agents (18/19)

---

## 📋 Implementation Checklist

### **Phase 1: Example Mapping (Specifying)**
- [ ] Set user story for BUG-031
- [ ] Add business rules (what must be true)
- [ ] Add concrete examples (Claude vs Cursor vs Aider scenarios)
- [ ] Add questions (any uncertainties)
- [ ] Generate scenarios from example map

### **Phase 2: Testing**
- [ ] Write test for Claude agent (should see ULTRATHINK)
- [ ] Write test for Cursor agent (should see generic language)
- [ ] Write test for Aider agent (should see generic language)
- [ ] Write test for default/unknown agent (safe fallback)
- [ ] Verify tests FAIL (red phase)

### **Phase 3: Implementation**
- [ ] Import `getAgentConfig` in `discover-foundation.ts`
- [ ] Add agent detection to `generateFieldReminder()`
- [ ] Replace hardcoded ULTRATHINK in initial draft reminder
- [ ] Replace hardcoded ULTRATHINK in project.vision field
- [ ] Update help documentation if needed
- [ ] Verify tests PASS (green phase)

### **Phase 4: Validation**
- [ ] Run all tests (ensure nothing broke)
- [ ] Test with `FSPEC_AGENT=claude` (should see ULTRATHINK)
- [ ] Test with `FSPEC_AGENT=cursor` (should NOT see ULTRATHINK)
- [ ] Run `npm run build` (ensure TypeScript compiles)
- [ ] Run `fspec validate` (ensure feature files valid)
- [ ] Update coverage mappings

---

## 🔗 Related Work Units

- **INIT-008** (done) - Agent runtime detection infrastructure
- **EXMAP-001** (done) - Example Mapping foundation
- **DISC-001** (likely related) - Discovery feedback loop

---

## 📊 Agent Capability Matrix

| Agent | ID | Supports Meta-Cognition | Supports System-Reminders | Category |
|-------|----|-----------------------|--------------------------|----------|
| Claude Code | `claude` | ✅ Yes | ✅ Yes | IDE |
| Cursor | `cursor` | ❌ No | ❌ No | IDE |
| Cline | `cline` | ❌ No | ❌ No | IDE |
| Aider | `aider` | ❌ No | ❌ No | CLI |
| Windsurf | `windsurf` | ❌ No | ❌ No | IDE |
| GitHub Copilot | `copilot` | ❌ No | ❌ No | Extension |
| Gemini | `gemini` | ❌ No | ❌ No | CLI |
| Qwen | `qwen` | ❌ No | ❌ No | CLI |
| *...11 more* | various | ❌ No | ❌ No | various |

**Key Insight**: Only 1 out of 19 agents (Claude Code) supports ULTRATHINK terminology. Fixing BUG-031 improves experience for **94.7%** of supported agents.

---

## 🧪 Test Scenarios to Implement

### **Scenario 1: Claude Agent Receives ULTRATHINK Guidance**
```gherkin
Given I am using Claude Code (FSPEC_AGENT=claude)
When I run fspec discover-foundation
Then I should see "ULTRATHINK: Read ALL code, understand deeply"
And I should see system-reminder tags
```

### **Scenario 2: Cursor Agent Receives Generic Guidance**
```gherkin
Given I am using Cursor (FSPEC_AGENT=cursor)
When I run fspec discover-foundation
Then I should see "Carefully analyze the entire codebase"
And I should NOT see "ULTRATHINK"
And I should see "**⚠️ IMPORTANT:**" format
```

### **Scenario 3: Aider Agent Receives Plain Text Guidance**
```gherkin
Given I am using Aider (FSPEC_AGENT=aider)
When I run fspec discover-foundation
Then I should see "Carefully analyze the entire codebase"
And I should NOT see "ULTRATHINK"
And I should see "**IMPORTANT:**" format (no emoji)
```

### **Scenario 4: Unknown Agent Receives Safe Default**
```gherkin
Given no agent is configured
When I run fspec discover-foundation
Then I should see "Carefully analyze the entire codebase"
And I should NOT see "ULTRATHINK"
And I should NOT see system-reminder tags
```

---

## 📝 Code Snippets Reference

### **Before (Hardcoded)**:
```typescript
const systemReminder = `Draft created. To complete foundation, you must ULTRATHINK the entire codebase.`;
```

### **After (Agent-Aware)**:
```typescript
const agent = getAgentConfig(cwd);
const thinkingInstruction = agent.supportsMetaCognition
  ? "you must ULTRATHINK the entire codebase"
  : "you must thoroughly analyze the entire codebase";

const systemReminder = `Draft created. To complete foundation, ${thinkingInstruction}.`;
```

---

## ✅ Success Criteria

**Definition of Done**:
1. ✅ Claude Code sees "ULTRATHINK" in discovery guidance
2. ✅ Non-Claude agents see generic "analyze" language
3. ✅ All agents receive appropriate output format (system-reminder vs bold text)
4. ✅ Tests cover Claude, IDE, and CLI agent types
5. ✅ No hardcoded ULTRATHINK remains in codebase
6. ✅ All existing tests still pass
7. ✅ Feature file scenarios pass validation
8. ✅ Coverage mappings updated

---

## 🚀 Next Steps

**Immediate Actions**:
1. ✅ Move BUG-031 to `specifying` status
2. ✅ Attach this investigation report to work unit
3. 🔄 Set user story for BUG-031
4. 🔄 Do Example Mapping (rules, examples, questions)
5. 🔄 Generate scenarios from example map
6. ➡️ Move to `testing` status
7. ➡️ Write failing tests
8. ➡️ Implement fix using Option 1
9. ➡️ Validate and mark done

**Estimated Timeline**: 1 work session (2-3 hours total)

---

## 📚 References

- **INIT-008 Implementation**: `src/utils/agentRuntimeConfig.ts`
- **Agent Registry**: `src/utils/agentRegistry.ts`
- **Discovery Command**: `src/commands/discover-foundation.ts`
- **Feature File**: `spec/features/implement-draft-driven-discovery-workflow-with-ai-chaining.feature`
- **Work Units**: `spec/work-units.json` (BUG-031, INIT-008)

---

**End of Investigation Report**
