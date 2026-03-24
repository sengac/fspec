# BRIDGE-019: AST Research — Bridge Server Patterns

## Research Summary

Analyzed the existing bridge codebase to understand patterns for the relay server implementation.

## Key Findings

### 1. WebSocket Server Pattern (from telegram-endpoint.ts)

The Telegram endpoint uses `WebSocketServer` from `ws` package as a server:

```typescript
import { WebSocketServer, WebSocket } from 'ws';
const wss = new WebSocketServer({ port, host });
wss.on('connection', (ws: WebSocket) => {
  ws.on('message', (data) => { /* handle */ });
  ws.on('close', () => { /* cleanup */ });
  ws.on('error', (err) => { /* log */ });
});
```

### 2. Relay Endpoint Architecture (from relay-endpoint.ts)

The relay endpoint is a **client** that connects to an upstream relay server:

```typescript
// Outbound: connects TO relay server
const ws = new WebSocket(endpointConfig.relayUrl);

// Inbound: accepts codelet connections locally
const wss = new WebSocketServer({ port });
```

This dual-WS architecture is why relay-endpoint.ts fails without a relay server.

### 3. Auth Protocol (from relay-endpoint.ts + fspec-mobile)

```
Client → Server: {type:'auth', data:{channel_id, api_key}}
Server → Client: {type:'authSuccess', data:{instances:[...]}}
  OR
Server → Client: {type:'authError', data:{code, message}}
```

### 4. Message Types (from relay-types.ts + fspec-mobile websocket_message.dart)

| Type | Direction | Purpose |
|------|-----------|---------|
| auth | client→server | Authentication handshake |
| authSuccess | server→client | Auth succeeded |
| authError | server→client | Auth failed |
| input | client→client | User text input |
| sessionControl | client→client | interrupt, clear |
| command | client→client | fspec CLI command request |
| commandResponse | client→client | fspec CLI command result |
| chunk | client→client | AI stream output |
| connected | client→client | New session announcement |
| ping | client→server | Heartbeat |
| pong | server→client | Heartbeat response |

### 5. State Management Pattern

relay-endpoint.ts uses a factory function with closure-scoped state:
```typescript
export function createRelayEndpoint(config): { /* methods */ } {
  const state = { /* private state */ };
  return { /* public API */ };
}
```

This is the preferred pattern for testability — the relay server should follow the same approach.

### 6. Fake Relay Server Reference (from fspec-mobile/tools/fake_relay_server.dart)

The Dart fake server shows the expected server-side behavior:
- Accepts WebSocket upgrades at any path
- Auth always succeeds (no key validation)
- Handles command/input/sessionControl/ping
- Returns hardcoded mock data for commands
- Does NOT route messages between clients (it's a test stub)

The real relay server must be a **pure router** — no mock data, just forwarding.
