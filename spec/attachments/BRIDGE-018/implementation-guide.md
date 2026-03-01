# BRIDGE-018: Relay Endpoint Command Flow Through Bridge WebSocket

## Objective

Correct the relay endpoint's command handling so it acts as a **pure protocol translator**. Currently, `relay-command-executor.ts` calls `fspecCallback` directly (wrong). Instead, commands from the relay should be translated to flat `InboundMessage` format and forwarded through the codelet's bridge WebSocket, where `bridge_relay.rs` (BRIDGE-017) handles the full `FspecCommandRequest → fspecCallback → FspecCommandResult → commandResponse` pipeline.

---

## What Changes

### 1. DELETE `bridge/relay-command-executor.ts`

This file directly imports and calls `fspecCallback`. It violates the architecture: the relay endpoint is a standalone process that should NOT have a dependency on `src/utils/fspec-callback`. Delete it entirely.

### 2. MODIFY `bridge/relay-endpoint.ts` — command handling

**Current (WRONG)** — lines 224-244:

```typescript
case 'command': {
    const cmdData = msg.data || {};
    const command = cmdData.command as string;
    const args = (cmdData.args as Record<string, unknown>) || {};
    const requestId = msg.request_id || '';
    const instanceId = msg.instance_id || '';

    void executeCommand(command, args).then(result => {
        sendToRelay({
            type: 'commandResponse',
            request_id: requestId,
            instance_id: instanceId,
            data: {
                command,
                success: result.success,
                result: result.result,
                error: result.error,
            },
        });
    });
    break;
}
```

**Corrected**: Translate to InboundMessage and forward to the session's codelet WebSocket:

```typescript
case 'command': {
    const sessionId = msg.session_id || '';
    const sessionWs = state.sessions.get(sessionId);
    if (!sessionWs) {
        console.warn(`${LOG_PREFIX} No codelet connection for session ${sessionId} (command)`);
        return;
    }
    
    const cmdData = msg.data || {};
    const fspecInbound = {
        type: 'command',
        session_id: sessionId,
        message: '',
        request_id: msg.request_id || '',
        command: cmdData.command as string,
        args_json: JSON.stringify(cmdData.args || {}),
    };
    
    sessionWs.send(JSON.stringify(fspecInbound));
    break;
}
```

This sends a flat InboundMessage with `type: "command"` through the bridge WebSocket. The codelet's `bridge_relay.rs` (BRIDGE-017) receives it and handles the full pipeline.

### 3. MODIFY `bridge/relay-endpoint.ts` — handle commandResponse from codelet

The codelet WebSocket handler currently does this (lines 256-277):

```typescript
function handleCodeletMessageInternal(ws: CodeletSocket, data: string): void {
    // ...
    if (msgType === 'connected') {
        // ...
    }
    // chunk + any other messages pass through to relay
    sendToRelay(msg);
}
```

This already forwards ALL messages (including `commandResponse`) from the codelet to the relay. **No change needed** — the passthrough behavior is correct. When `bridge_relay.rs` sends a `commandResponse` OutboundMessage, the codelet WS handler receives it and forwards it to the relay.

### 4. MODIFY `bridge/relay-endpoint.ts` — remove imports

Remove:
```typescript
import { executeCommand } from './relay-command-executor';
```

And the re-export:
```typescript
export { executeCommand };
```

### 5. UPDATE `bridge/relay-types.ts`

The `FspecInboundMessage` interface needs command fields (may already be there but verify):

```typescript
export interface FspecInboundMessage {
    type: string;
    session_id: string;
    message?: string;
    images?: Array<{ data: string; media_type: string }>;
    action?: string;
    response?: string;
    // BRIDGE-018: Command fields
    request_id?: string;
    command?: string;
    args_json?: string;
}
```

---

## Test Changes

### `bridge/__tests__/relay-endpoint.test.ts`

#### Scenario: Execute fspec command via StreamChunk pipeline

The test currently waits for `executeCommand` to complete and checks `commandResponse`. With the corrected architecture, the test should verify:
1. Command message from relay → translated to InboundMessage → sent to codelet WS
2. (The Rust pipeline is tested in BRIDGE-017)
3. commandResponse from codelet WS → forwarded to relay

```typescript
describe('Scenario: Execute fspec command via StreamChunk pipeline and return result', () => {
    it('should translate command to InboundMessage and forward to codelet WS', () => {
        const config: RelayEndpointConfig = { /* ... */ };
        const endpoint = createRelayEndpoint(config);
        endpoint.handleRelayMessage(JSON.stringify({ type: 'authSuccess', data: {} }));
        
        // Connect a codelet session
        const mockCodeletWs = createMockWebSocket();
        endpoint.handleCodeletMessage(mockCodeletWs, JSON.stringify({
            type: 'connected',
            session_id: 'session-X',
            data: {},
        }));
        
        // Relay sends a command
        endpoint.handleRelayMessage(JSON.stringify({
            type: 'command',
            session_id: 'session-X',
            request_id: 'req-001',
            data: { command: 'board', args: {} },
        }));
        
        // Verify the codelet WS received a translated InboundMessage
        expect(mockCodeletWs.sentMessages.length).toBe(1);
        const sent = JSON.parse(mockCodeletWs.sentMessages[0]);
        expect(sent.type).toBe('command');
        expect(sent.session_id).toBe('session-X');
        expect(sent.request_id).toBe('req-001');
        expect(sent.command).toBe('board');
        expect(sent.args_json).toBe('{}');
    });
});
```

#### Scenario: Forward commandResponse from codelet to relay

```typescript
describe('Scenario: commandResponse from codelet forwarded to relay', () => {
    it('should pass through commandResponse without transformation', () => {
        const config: RelayEndpointConfig = { /* ... */ };
        const endpoint = createRelayEndpoint(config);
        endpoint.handleRelayMessage(JSON.stringify({ type: 'authSuccess', data: {} }));
        
        const mockCodeletWs = createMockWebSocket();
        endpoint.handleCodeletMessage(mockCodeletWs, JSON.stringify({
            type: 'connected', session_id: 'session-X', data: {},
        }));
        
        // Codelet sends a commandResponse (from bridge_relay.rs)
        endpoint.handleCodeletMessage(mockCodeletWs, JSON.stringify({
            type: 'commandResponse',
            session_id: 'session-X',
            request_id: 'req-001',
            data: { command: 'board', success: true, result: { columns: {} } },
        }));
        
        // Verify it was forwarded to relay
        const sent = endpoint.getRelaySentMessages();
        const response = sent.find(m => m.type === 'commandResponse');
        expect(response).toBeDefined();
        expect(response.request_id).toBe('req-001');
    });
});
```

#### Scenario: FspecCommandResult chunks are intercepted

This scenario is about Rust behavior (BRIDGE-017), not the TypeScript endpoint. The TS test should verify that the relay endpoint does NOT see `fspecCommandResult` chunks (because bridge_relay.rs intercepts them). In unit tests with mocks, this can be verified by ensuring that `fspecCommandResult` type chunks from the codelet are NOT forwarded:

Actually, `fspecCommandResult` would never appear in messages from the codelet WS — `bridge_relay.rs` intercepts them and converts to `commandResponse` before sending. So the TS endpoint never sees `fspecCommandResult`. The test already covers this implicitly by testing that `commandResponse` (not `fspecCommandResult`) arrives.

#### Scenarios: Command timeout, unknown command

These are now handled entirely by the Rust pipeline (BRIDGE-017). The TS endpoint just forwards commands and receives responses. The timeout/error tests should verify that the endpoint correctly forwards `commandResponse` messages with `success: false`.

---

## What to Remove from relay-endpoint.ts Exports

Remove these re-exports:
```typescript
import { executeCommand } from './relay-command-executor';
export { executeCommand };
```

The test file should also stop importing `executeCommand`.

---

## Verification

1. `npm test -- bridge/__tests__/relay-endpoint.test.ts` — all tests pass
2. `npx tsc --noEmit` — no TypeScript errors
3. `relay-command-executor.ts` is deleted
4. No imports from `src/utils/fspec-callback` anywhere in `bridge/` directory
5. Manual test with fake relay: send command → verify InboundMessage reaches codelet WS

---

## Estimate: 3 points

The main work is updating tests. The code change is small (replace `executeCommand()` call with WebSocket send). Deleting relay-command-executor.ts is straightforward.
