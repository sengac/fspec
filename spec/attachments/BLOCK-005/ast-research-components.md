# AST Research: Sensitive Path Prompts (BLOCK-005)

## Key Findings

### 1. ConfirmationDialog Component

**File:** `src/components/ConfirmationDialog.tsx`

**Current ConfirmMode Type (Line 25):**
```typescript
type ConfirmMode = 'yesno' | 'typed' | 'keypress' | 'visual';
```

**Props Interface (Line 28):**
```typescript
interface ConfirmationDialogProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmMode?: ConfirmMode;
  typedPhrase?: string;
  riskLevel?: RiskLevel;
  description?: string;
}
```

**Implementation Approach:**
- Add `'triple'` to ConfirmMode union type
- Add `onTripleConfirm?: (choice: 'allowOnce' | 'allowSession' | 'deny') => void` to props
- Add state for three-button selection: `useState<'allowOnce' | 'allowSession' | 'deny'>('allowOnce')`
- Implement ←/→ navigation between three buttons
- Use existing visual mode pattern as reference

### 2. Blocklist Matcher

**File:** `codelet/tools/src/blocklist/matcher.rs`

**check_command Method (Line 84):**
```rust
pub fn check_command(&self, command: &str) -> CheckResult {
    // Currently handles Block, Allow, and Prompt actions
    // Prompt action returns: allowed=false, blocked=false
}
```

### 3. Session Allowances (To Be Added)

**File:** `codelet/tools/src/blocklist/middleware.rs`

**Required Changes:**
```rust
static SESSION_ALLOWANCES: RwLock<HashSet<String>> = RwLock::new(HashSet::new());

pub fn allow_for_session(pattern: &str) {
    let mut guard = SESSION_ALLOWANCES.write().unwrap();
    guard.insert(pattern.to_string());
}

pub fn is_session_allowed(pattern: &str) -> bool {
    let guard = SESSION_ALLOWANCES.read().unwrap();
    guard.contains(pattern)
}

pub fn clear_session_allowances() {
    let mut guard = SESSION_ALLOWANCES.write().unwrap();
    guard.clear();
}
```

### 4. NAPI Binding

**File:** `codelet/napi/src/blocklist.rs`

**Required Addition:**
```rust
#[napi]
pub fn blocklist_allow_session(pattern: String) -> Result<()> {
    codelet_tools::blocklist::allow_for_session(&pattern);
    Ok(())
}
```

## Integration Points

1. **ConfirmationDialog** - Extend with triple mode
2. **middleware.rs** - Add session allowances storage and checking
3. **blocklist.rs (NAPI)** - Expose `blocklist_allow_session` to TypeScript
4. **AgentView.tsx** - Handle prompt result and show triple dialog
5. **BashToolFacadeWrapper** - Check session allowances before returning prompt result

## Files to Modify

- `src/components/ConfirmationDialog.tsx`
- `codelet/tools/src/blocklist/middleware.rs`
- `codelet/napi/src/blocklist.rs`
- `codelet/napi/index.d.ts` (auto-generated)
