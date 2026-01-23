# AST Research: Implementation Patterns for WATCH-023

## Research Objective
Analyze existing codebase patterns to guide implementation of Watcher Templates feature.

---

## 1. Flat List Navigation Pattern (WatcherTemplateList)

### 1.1 buildFlatModelList Function Location
src/tui/components/AgentView.tsx:174:1:const buildFlatModelList = (

### 1.2 Relevant Pattern Context (Lines 174-228)
```typescript
// Union type pattern for flat list items
type ModelListItem = 
  | { type: 'provider'; provider: ProviderSection; isExpanded: boolean }
  | { type: 'model'; provider: ProviderSection; model: ModelOption };

// Build flat list from hierarchical data
const buildFlatModelList = (
  providerSections: ProviderSection[],
  expandedProviders: Set<string>
): ModelListItem[] => {
  const items: ModelListItem[] = [];
  providerSections.forEach(provider => {
    const isExpanded = expandedProviders.has(provider.providerId);
    items.push({ type: 'provider', provider, isExpanded });
    if (isExpanded) {
      provider.models.forEach(model => {
        items.push({ type: 'model', provider, model });
      });
    }
  });
  return items;
};
```

**Key Takeaways:**
- Union type for flat list items (template | instance | create-new)
- Expansion state tracked via Set<string>
- Build function returns union array for navigation

---

## 2. Config Storage Pattern (watcherTemplateStorage)

### 2.1 getFspecUserDir Function Location
No matches found

### 2.2 loadConfig Pattern Location
No matches found

### 2.3 writeConfig Pattern Location
No matches found

### 2.4 Relevant Pattern Context
```typescript
// User directory location
export function getFspecUserDir(): string {
  return join(homedir(), '.fspec');
}

// Async load with fallback
export async function loadConfig(): Promise<FspecConfig | null> {
  const userConfigPath = join(getFspecUserDir(), 'fspec-config.json');
  try {
    const content = await readFile(userConfigPath, 'utf-8');
    return JSON.parse(content);
  } catch {
    return null;
  }
}

// Sync write with directory creation
export function writeConfig(config: FspecConfig): void {
  const userConfigPath = join(getFspecUserDir(), 'fspec-config.json');
  mkdirSync(dirname(userConfigPath), { recursive: true });
  writeFileSync(userConfigPath, JSON.stringify(config, null, 2));
}
```

**Key Takeaways:**
- Use `getFspecUserDir()` for user-level storage path
- Async load with try/catch fallback to null
- Sync write with `mkdirSync` for directory creation
- JSON.parse/JSON.stringify for serialization

---

## 3. Form Component Pattern (WatcherTemplateForm)

### 3.1 WatcherCreateView Exports
No matches found

### 3.2 WatcherCreateView Props
No matches found

### 3.3 WatcherCreateView Function Component
No matches found

### 3.4 Relevant Pattern Context
The existing WatcherCreateView.tsx provides:
- Props pattern: `{ parentSessionId, currentModel, onCreateWatcher, onCancel }`
- Form state management with useState for each field
- Field focus tracking with `focusedField` state
- Tab/Shift+Tab navigation between fields
- useInput hook for keyboard handling

**Key Takeaways:**
- Refactor to WatcherTemplateForm with `mode: 'create' | 'edit'` prop
- Add `template?: WatcherTemplate` prop for edit mode pre-population
- Keep form field pattern but add arrow key navigation (↑/↓)

---

## 4. Confirmation Dialog Pattern

### 4.1 ConfirmationDialog Interface
No matches found

### 4.2 Relevant Pattern Context
```typescript
export interface ConfirmationDialogProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmMode?: 'yesno' | 'typed' | 'keypress';
  typedPhrase?: string;
  riskLevel?: 'low' | 'medium' | 'high';
  description?: string;
}
```

**Key Takeaways:**
- Use `riskLevel='medium'` for template deletion with active instances
- Use `riskLevel='low'` for simple deletions
- `description` prop for warning about active instances

---

## 5. Type Definition Pattern (types/watcherTemplate.ts)

### 5.1 Existing Conversation Types
No matches found

### 5.2 Proposed Type Definitions
```typescript
// src/tui/types/watcherTemplate.ts

export interface WatcherTemplate {
  id: string;                         // UUID
  name: string;                       // "Security Reviewer"
  slug: string;                       // "security-reviewer" (auto-generated)
  modelId: string;                    // "anthropic/claude-sonnet-4-20250514"
  authority: 'peer' | 'supervisor';
  brief: string;                      // Watching instructions
  autoInject: boolean;
  createdAt: string;                  // ISO timestamp
  updatedAt: string;                  // ISO timestamp
}

export interface WatcherInstance {
  sessionId: string;                  // UUID of the watcher session
  templateId: string;                 // Which template it was spawned from
  status: 'running' | 'idle';
}

// Union type for flat list navigation
export type WatcherListItem =
  | { type: 'template'; template: WatcherTemplate; isExpanded: boolean; instanceCount: number }
  | { type: 'instance'; template: WatcherTemplate; instance: WatcherInstance }
  | { type: 'create-new' };
```

---

## 6. Test Pattern Analysis

### 6.1 Existing Watcher Test Pattern
No matches found

### 6.2 Test Organization Pattern
```typescript
// Feature file reference at top of test file
/**
 * Feature: spec/features/watcher-templates.feature
 * Tests for watcher templates (WATCH-023)
 */

// Mock the NAPI module
vi.mock('@sengac/codelet-napi', () => ({ ... }));

// Scenario-based describe blocks
describe('Feature: Watcher Templates', () => {
  describe('Scenario: View templates with active instance count', () => {
    it('should display template with instance count badge', () => {
      // @step Given I have a "Security Reviewer" template with 2 active instances
      // @step When I open the /watcher overlay
      // @step Then I should see "Security Reviewer (Supervisor)" with "[2 active]"
    });
  });
});
```

---

## 7. Slug Generation Pattern

### 7.1 Kebab-case Implementation
```typescript
// generateSlug function for watcherTemplateStorage.ts
export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')  // Remove special chars except spaces/hyphens
    .replace(/\s+/g, '-')          // Replace spaces with hyphens
    .replace(/-+/g, '-')           // Collapse multiple hyphens
    .replace(/^-|-$/g, '');        // Trim leading/trailing hyphens
}

// Examples:
// "Security Reviewer" → "security-reviewer"
// "Code Review & Analysis" → "code-review-analysis"
// "Test  Enforcer" → "test-enforcer"
```

---

## Summary: Files to Create/Modify

| File | Action | Pattern Source |
|------|--------|----------------|
| `src/tui/types/watcherTemplate.ts` | CREATE | conversation.ts |
| `src/tui/utils/watcherTemplateStorage.ts` | CREATE | config.ts |
| `src/tui/components/WatcherTemplateList.tsx` | CREATE | AgentView.tsx (buildFlatModelList) |
| `src/tui/components/WatcherTemplateForm.tsx` | REFACTOR | WatcherCreateView.tsx |
| `src/tui/components/AgentView.tsx` | MODIFY | Existing patterns |

---

## Test Files to Create

| Test File | Tests For |
|-----------|-----------|
| `src/tui/utils/__tests__/watcherTemplateStorage.test.ts` | Storage + slug generation |
| `src/tui/components/__tests__/watcher-template-list.test.tsx` | List navigation + display |
| `src/tui/components/__tests__/watcher-template-form.test.tsx` | Form logic + validation |
