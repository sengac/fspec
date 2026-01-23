# AST Research: Auto-Inject Toggle Implementation

## WATCH-021: Auto-Inject Toggle in Watcher Creation Dialog

### Files to Modify

#### 1. WatcherCreateView.tsx (src/tui/components/WatcherCreateView.tsx)

**Current Focus Order:**
```typescript
type FocusField = 'name' | 'authority' | 'model' | 'brief' | 'create';
const FOCUS_ORDER: FocusField[] = ['name', 'authority', 'model', 'brief', 'create'];
```

**Required Changes:**
```typescript
type FocusField = 'name' | 'authority' | 'model' | 'brief' | 'autoInject' | 'create';
const FOCUS_ORDER: FocusField[] = ['name', 'authority', 'model', 'brief', 'autoInject', 'create'];
```

**Current onCreate Signature (line 39):**
```typescript
onCreate: (name: string, authority: 'peer' | 'supervisor', model: string, brief: string) => void;
```

**Required Signature:**
```typescript
onCreate: (name: string, authority: 'peer' | 'supervisor', model: string, brief: string, autoInject: boolean) => void;
```

#### 2. session_manager.rs (codelet/napi/src/session_manager.rs)

**Current session_set_role (line 3726):**
```rust
pub fn session_set_role(
    session_id: String,
    role_name: String,
    role_description: Option<String>,
    authority: String,
) -> Result<()>
```

**Required Signature:**
```rust
pub fn session_set_role(
    session_id: String,
    role_name: String,
    role_description: Option<String>,
    authority: String,
    auto_inject: Option<bool>,  // NEW: defaults to true if None
) -> Result<()>
```

#### 3. AgentView.tsx (src/tui/components/AgentView.tsx)

**Current handleWatcherCreate (line 4257):**
```typescript
const handleWatcherCreate = useCallback(async (
  name: string,
  authority: 'peer' | 'supervisor',
  model: string,
  brief: string
) => {
```

**Required Signature:**
```typescript
const handleWatcherCreate = useCallback(async (
  name: string,
  authority: 'peer' | 'supervisor',
  model: string,
  brief: string,
  autoInject: boolean  // NEW
) => {
```

**Current sessionSetRole call (line 4277):**
```typescript
sessionSetRole(
  watcherId,
  name.trim(),
  brief.trim() || null,
  authority
);
```

**Required call:**
```typescript
sessionSetRole(
  watcherId,
  name.trim(),
  brief.trim() || null,
  authority,
  autoInject  // NEW
);
```

### UI Component Pattern (Authority Selector Reference)

The Auto-inject toggle should follow the same pattern as the Authority selector:

```typescript
{/* Authority selector (current implementation) */}
<Box marginBottom={1} flexDirection="column">
  <Text color={focusField === 'authority' ? 'cyan' : 'white'}>
    Authority:
  </Text>
  <Box>
    <Text
      backgroundColor={focusField === 'authority' && authority === 'peer' ? 'blue' : undefined}
      color={authority === 'peer' ? 'green' : 'gray'}
    >
      [{authority === 'peer' ? '●' : ' '}] Peer
    </Text>
    ...
    {focusField === 'authority' && (
      <Text dimColor> (←/→ to toggle)</Text>
    )}
  </Box>
</Box>
```

### Auto-inject Toggle UI Design

```
Auto-inject:
[●] Enabled  (green when enabled, gray when disabled)
             (←/→ to toggle) hint when focused
```

When disabled:
```
Auto-inject:
[ ] Disabled (gray)
```

### Keyboard Handling

```typescript
case 'autoInject':
  // Left/Right toggles auto-inject
  if (key.leftArrow || key.rightArrow) {
    setAutoInject(prev => !prev);
  }
  break;
```

### Summary

1. Add `autoInject` state with default `true`
2. Add `'autoInject'` to FocusField type and FOCUS_ORDER array
3. Add keyboard handler for autoInject field (Left/Right toggles)
4. Update onCreate callback signature
5. Update NAPI session_set_role to accept optional auto_inject
6. Update AgentView handleWatcherCreate to pass autoInject
