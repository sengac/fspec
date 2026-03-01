# AST Research: BRIDGE-018 Relay Command Flow

## 1. Import of executeCommand in relay-endpoint.ts

```
bridge/relay-endpoint.ts:35:1: import { executeCommand } from './relay-command-executor';
```

This import and its re-export (line 43) must be removed.

## 2. Test imports from relay-endpoint.ts

```
bridge/__tests__/relay-endpoint.test.ts:11:1: import {
  validateConfig,
  translateRelayInputToFspec,
  translateRelaySessionControlToFspec,
  executeCommand,
  createRelayEndpoint,
  type RelayEndpointConfig,
  type RelayMessage,
} from '../relay-endpoint';
```

The `executeCommand` import must be removed from tests.

## 3. fspecCallback usage in bridge/ directory

```
bridge/relay-command-executor.ts:37: const { fspecCallback } = await import('../src/utils/fspec-callback');
```

This is the ONLY fspecCallback import in bridge/ — it will be eliminated by deleting relay-command-executor.ts.

## 4. Files requiring changes

- **DELETE**: `bridge/relay-command-executor.ts` — calls fspecCallback directly (wrong architecture)
- **MODIFY**: `bridge/relay-endpoint.ts` — remove executeCommand import/export, rewrite command case to forward to codelet WS
- **MODIFY**: `bridge/relay-types.ts` — add command fields to FspecInboundMessage, remove unused CommandResult/CommandResponseMessage
- **MODIFY**: `bridge/__tests__/relay-endpoint.test.ts` — remove executeCommand import, rewrite command/timeout tests

## 5. Codelet message handler (passthrough)

The `handleCodeletMessageInternal` function at line 256 of relay-endpoint.ts already passes through ALL messages from codelet to relay via `sendToRelay(msg)`. This means commandResponse messages from bridge_relay.rs will automatically be forwarded to the relay — no change needed for this direction.
