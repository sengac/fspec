# AST Research — AMGR-012 Role Management

## 1. Base Dialog Component

```
No matches found
src/components/Dialog.tsx:35:8:const Dialog: React.FC<DialogProps> = ({
```

## 2. Existing Dialog Components (pattern reference)

```
src/components/CreateSessionDialog.tsx:52:8:const CreateSessionDialog: React.FC<CreateSessionDialogProps> = ({
src/tui/components/ThinkingLevelDialog.tsx:64:8:const ThinkingLevelDialog: React.FC<ThinkingLevelDialogProps> = ({
src/components/ConfirmationDialog.tsx:44:8:const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
src/tui/components/AttachmentDialog.tsx:27:8:const AttachmentDialog: React.FC<AttachmentDialogProps> = ({
```

## 3. useMultiLineInput Hook

```
src/tui/hooks/useMultiLineInput.ts:58:8:function useMultiLineInput(
No matches found
No matches found
```

## 4. NAPI session_set_role and session_get_role

```
codelet/napi/src/session_manager.rs:5526:1:pub fn session_set_role(
codelet/napi/src/session_manager.rs:5542:1:pub fn session_get_role(session_id: String) -> Result<Option<SupervisorRoleInfo>> {
```

## 5. AgentManager Types (action enum)

```
No matches found
No matches found
```

## 6. AgentManager Handler

```
codelet/napi/src/agent_manager_handler.rs:30:1:pub fn create_handler(
codelet/napi/src/agent_manager_handler.rs:145:1:fn handle_list(session_manager: &SessionManager) -> AgentManagerResult {
codelet/napi/src/agent_manager_handler.rs:188:1:fn handle_get_status(
```

## 7. AgentManager Tool Definition (mod.rs)

```
No matches found
```

## 8. Slash Command Handling in AgentView

```
No matches found
```

## 9. Input Priority System

```
src/tui/input/types.ts:27:8:const InputPriority = {
```

## Summary

- Dialog base: src/components/Dialog.tsx — provides modal overlay, ESC handling, border
- useMultiLineInput hook: src/tui/hooks/useMultiLineInput.ts — cursor, editing, viewport scrolling
- session_set_role: codelet/napi/src/session_manager.rs:5526 — (session_id, role_name, _role_brief, _auto_inject)
- session_get_role: codelet/napi/src/session_manager.rs:5542 — returns Option<SupervisorRoleInfo>
- AgentManagerAction enum: codelet/tools/src/agent_manager/types.rs — needs SetRole variant
- Handler: codelet/napi/src/agent_manager_handler.rs — needs handle_set_role fn
- /role integration: src/tui/components/AgentView.tsx — following /thinking pattern
