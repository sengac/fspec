# TUI-016: Cross-Platform IPC Architecture for Checkpoint Updates

## Overview

This document describes the architecture for replacing file-watching (chokidar) with cross-platform IPC (Inter-Process Communication) to update checkpoint counts in the TUI when checkpoint commands execute.

## Problem Statement

Currently, `CheckpointPanel` uses chokidar to watch `.git/fspec-checkpoints-index/` for file changes. This should be replaced with command-triggered updates using IPC, eliminating file-watching overhead while maintaining real-time updates.

## Solution: Native Node.js `net` Module for Cross-Platform IPC

### Why Native `net` Module?

- ✅ **Zero dependencies** - Built into Node.js
- ✅ **Cross-platform** - Automatically handles Unix sockets (Linux/Mac) and Named Pipes (Windows)
- ✅ **Fast** - Bypasses network stack entirely (fastest IPC method)
- ✅ **Safe** - No third-party security vulnerabilities (node-ipc had CVE-2022-23812)
- ✅ **Simple** - Same API across all platforms

### Platform-Specific Implementation

| Platform | Transport | Path Format | Cleanup |
|----------|-----------|-------------|---------|
| Linux/Mac | Unix Domain Socket | `/tmp/fspec-tui.sock` | Manual unlink required |
| Windows | Named Pipe | `\\\\.\\pipe\\fspec-tui` | Auto-cleanup on close |

## Architecture

### 1. Shared IPC Utility (`src/utils/ipc.ts`)

Create a shared utility module that provides cross-platform IPC server and client functionality.

```typescript
/**
 * Cross-platform IPC utility for fspec
 * Uses Unix domain sockets (Linux/Mac) and Named Pipes (Windows)
 */

import net from 'net';
import fs from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

export interface IPCMessage {
  type: string;
  payload?: Record<string, unknown>;
}

/**
 * Get cross-platform IPC path
 * - Windows: Named pipe (\\.\pipe\fspec-tui)
 * - Unix: Socket file (/tmp/fspec-tui.sock)
 */
export function getIPCPath(name = 'fspec-tui'): string {
  if (process.platform === 'win32') {
    return `\\\\.\\pipe\\${name}`;
  }
  return join(tmpdir(), `${name}.sock`);
}

/**
 * Create IPC server for receiving messages
 * Used by TUI to listen for checkpoint updates
 */
export function createIPCServer(
  onMessage: (message: IPCMessage) => void
): net.Server {
  const pipePath = getIPCPath();

  // Cleanup existing socket on Unix (Windows auto-cleans)
  if (process.platform !== 'win32' && fs.existsSync(pipePath)) {
    try {
      fs.unlinkSync(pipePath);
    } catch (error) {
      // Ignore cleanup errors
    }
  }

  const server = net.createServer((client) => {
    client.on('data', (data) => {
      try {
        const message: IPCMessage = JSON.parse(data.toString());
        onMessage(message);
      } catch (error) {
        // Ignore malformed messages
      }
    });

    client.on('error', () => {
      // Ignore client errors
    });
  });

  server.on('error', (error) => {
    // Ignore server errors (e.g., address already in use)
  });

  return server;
}

/**
 * Send IPC message to server
 * Used by checkpoint commands to notify TUI
 * Fails silently if TUI is not running
 */
export function sendIPCMessage(message: IPCMessage): void {
  const pipePath = getIPCPath();

  const client = net.connect(pipePath, () => {
    client.write(JSON.stringify(message));
    client.end();
  });

  // Fail silently if TUI not running
  client.on('error', () => {
    // TUI not running, ignore
  });
}

/**
 * Cleanup IPC server resources
 * Call this when TUI exits
 */
export function cleanupIPCServer(server: net.Server): void {
  server.close();

  const pipePath = getIPCPath();

  // Manual cleanup for Unix sockets (Windows auto-cleans)
  if (process.platform !== 'win32' && fs.existsSync(pipePath)) {
    try {
      fs.unlinkSync(pipePath);
    } catch (error) {
      // Ignore cleanup errors
    }
  }
}
```

### 2. TUI Integration (`src/tui/components/BoardView.tsx`)

The TUI starts an IPC server on mount and closes it on unmount.

```typescript
import { createIPCServer, cleanupIPCServer } from '../../utils/ipc';

// Inside BoardView component
useEffect(() => {
  const server = createIPCServer((message) => {
    if (message.type === 'checkpoint-changed') {
      // Trigger zustand store refresh
      useFspecStore.getState().loadCheckpointCounts();
    }
  });

  server.listen(getIPCPath(), () => {
    // IPC server ready
  });

  return () => {
    cleanupIPCServer(server);
  };
}, []);
```

### 3. Zustand Store Integration (`src/tui/store/fspecStore.ts`)

Add `checkpointCounts` state and `loadCheckpointCounts()` action to the zustand store.

```typescript
interface CheckpointCounts {
  manual: number;
  auto: number;
}

interface FspecState {
  // ... existing state
  checkpointCounts: CheckpointCounts;

  // Actions
  loadCheckpointCounts: () => Promise<void>;
}

export const useFspecStore = create<FspecState>()(
  immer((set, get) => ({
    // ... existing state
    checkpointCounts: { manual: 0, auto: 0 },

    loadCheckpointCounts: async () => {
      const cwd = get().cwd;
      const counts = await countCheckpoints(cwd);
      set(state => {
        state.checkpointCounts = counts;
      });
    },
  }))
);

/**
 * Count checkpoints by reading index files
 * Moved from CheckpointPanel to be reusable
 */
async function countCheckpoints(cwd: string): Promise<CheckpointCounts> {
  const indexDir = join(cwd, '.git', 'fspec-checkpoints-index');

  let manual = 0;
  let auto = 0;

  try {
    if (!fs.existsSync(indexDir)) {
      return { manual: 0, auto: 0 };
    }

    const files = fs.readdirSync(indexDir).filter(f => f.endsWith('.json'));

    for (const file of files) {
      const filePath = join(indexDir, file);
      const content = fs.readFileSync(filePath, 'utf-8');
      const data = JSON.parse(content) as {
        checkpoints: Array<{ name: string; message: string }>
      };

      for (const checkpoint of data.checkpoints || []) {
        if (checkpoint.name.includes('-auto-')) {
          auto++;
        } else {
          manual++;
        }
      }
    }
  } catch (error) {
    // Silent failure - return zero counts if error
  }

  return { manual, auto };
}
```

### 4. CheckpointPanel Refactor (`src/tui/components/CheckpointPanel.tsx`)

Remove chokidar, read from zustand store instead.

```typescript
import { useFspecStore } from '../store/fspecStore';

export const CheckpointPanel: React.FC<CheckpointPanelProps> = ({ cwd }) => {
  // Read from zustand store (no local state, no file watching)
  const checkpointCounts = useFspecStore(state => state.checkpointCounts);
  const loadCheckpointCounts = useFspecStore(state => state.loadCheckpointCounts);

  // Load counts on mount
  useEffect(() => {
    void loadCheckpointCounts();
  }, [loadCheckpointCounts]);

  // Format display
  const displayText = checkpointCounts.manual === 0 && checkpointCounts.auto === 0
    ? 'Checkpoints: None'
    : `Checkpoints: ${checkpointCounts.manual} Manual, ${checkpointCounts.auto} Auto`;

  return (
    <Box flexDirection="column">
      <Text>{displayText}</Text>
    </Box>
  );
};
```

### 5. Checkpoint Command Integration

All checkpoint commands send IPC notifications after modifying checkpoint state.

#### Manual Checkpoint (`src/commands/checkpoint.ts`)

```typescript
import { sendIPCMessage } from '../utils/ipc';

export async function checkpoint(options: CheckpointOptions): Promise<{...}> {
  // ... existing checkpoint creation logic

  const result = await createCheckpointUtil({...});

  // Notify TUI after checkpoint created
  sendIPCMessage({ type: 'checkpoint-changed' });

  return result;
}
```

#### Automatic Checkpoint (`src/commands/update-work-unit-status.ts`)

```typescript
import { sendIPCMessage } from '../utils/ipc';

// After creating automatic checkpoint (line ~415)
await gitCheckpoint.createCheckpoint({...});

// Notify TUI
sendIPCMessage({ type: 'checkpoint-changed' });
```

#### Restore Checkpoint (`src/commands/restore-checkpoint.ts`)

```typescript
import { sendIPCMessage } from '../utils/ipc';

export async function restoreCheckpoint(options: RestoreOptions): Promise<{...}> {
  // ... existing restore logic

  // Notify TUI after restoration (counts may change due to index updates)
  sendIPCMessage({ type: 'checkpoint-changed' });

  return result;
}
```

#### Cleanup Checkpoints (`src/commands/cleanup-checkpoints.ts`)

```typescript
import { sendIPCMessage } from '../utils/ipc';

export async function cleanupCheckpoints(options: CleanupOptions): Promise<{...}> {
  // ... existing cleanup logic

  // Notify TUI after cleanup
  sendIPCMessage({ type: 'checkpoint-changed' });

  return result;
}
```

## Implementation Checklist

### Phase 1: Create Shared IPC Utility
- [ ] Create `src/utils/ipc.ts` with cross-platform IPC functions
- [ ] Add unit tests for `getIPCPath()` on different platforms
- [ ] Add integration tests for server/client communication

### Phase 2: Zustand Store Integration
- [ ] Add `checkpointCounts` state to `fspecStore.ts`
- [ ] Add `loadCheckpointCounts()` action
- [ ] Move `countCheckpoints()` logic from `CheckpointPanel` to store
- [ ] Add tests for zustand store checkpoint loading

### Phase 3: TUI Integration
- [ ] Add IPC server to `BoardView.tsx` (mount/unmount lifecycle)
- [ ] Refactor `CheckpointPanel.tsx` to read from zustand store
- [ ] Remove chokidar dependency from `CheckpointPanel.tsx`
- [ ] Remove chokidar watcher useEffect
- [ ] Update tests for `CheckpointPanel` (no file watching, store-based)

### Phase 4: Command Integration
- [ ] Add IPC notification to `checkpoint.ts` (manual checkpoints)
- [ ] Add IPC notification to `update-work-unit-status.ts` (auto checkpoints)
- [ ] Add IPC notification to `restore-checkpoint.ts`
- [ ] Add IPC notification to `cleanup-checkpoints.ts`
- [ ] Add integration tests for command → IPC → TUI flow

### Phase 5: Cleanup
- [ ] Remove chokidar from `package.json` (if no other usages)
- [ ] Update feature file with new architecture
- [ ] Update coverage file with test mappings

## Testing Strategy

### Unit Tests

```typescript
// src/utils/__tests__/ipc.test.ts
describe('IPC Utility', () => {
  it('should return Windows named pipe path on win32', () => {
    const originalPlatform = process.platform;
    Object.defineProperty(process, 'platform', { value: 'win32' });
    expect(getIPCPath()).toBe('\\\\.\\pipe\\fspec-tui');
    Object.defineProperty(process, 'platform', { value: originalPlatform });
  });

  it('should return Unix socket path on Linux/Mac', () => {
    const originalPlatform = process.platform;
    Object.defineProperty(process, 'platform', { value: 'linux' });
    expect(getIPCPath()).toMatch(/\/tmp\/fspec-tui\.sock/);
    Object.defineProperty(process, 'platform', { value: originalPlatform });
  });

  it('should send message and receive on server', (done) => {
    const server = createIPCServer((message) => {
      expect(message.type).toBe('test');
      cleanupIPCServer(server);
      done();
    });

    server.listen(getIPCPath(), () => {
      sendIPCMessage({ type: 'test' });
    });
  });
});
```

### Integration Tests

```typescript
// src/commands/__tests__/checkpoint-ipc.test.ts
describe('Checkpoint IPC Integration', () => {
  it('should notify TUI when checkpoint created', async () => {
    // Mock IPC server
    const messages: IPCMessage[] = [];
    const server = createIPCServer((msg) => messages.push(msg));
    server.listen(getIPCPath());

    // Create checkpoint
    await checkpoint({
      workUnitId: 'TEST-001',
      checkpointName: 'baseline',
      cwd: testDir,
    });

    // Wait for IPC message
    await new Promise(resolve => setTimeout(resolve, 100));

    expect(messages).toContainEqual({ type: 'checkpoint-changed' });
    cleanupIPCServer(server);
  });
});
```

## Security Considerations

- **Unix sockets** - Only accessible to processes running as same user (file permissions)
- **Windows named pipes** - Default security descriptor restricts access to same user
- **Malformed messages** - Parser wrapped in try-catch, silently ignores bad JSON
- **DOS attacks** - Not a concern (local IPC, trusted processes only)

## Performance Characteristics

- **Connection overhead** - ~1-2ms per message (negligible)
- **Message size** - Tiny JSON payloads (~50 bytes)
- **Frequency** - Low (only when checkpoints created/deleted)
- **Compared to chokidar** - Much lower CPU/memory overhead (no file watching)

## Fallback Behavior

If TUI is not running:
- `sendIPCMessage()` fails silently (connection refused)
- Checkpoint commands complete successfully
- Next time TUI opens, counts are loaded fresh from disk

## Migration Path

1. Implement IPC utility and tests
2. Add zustand store actions
3. Update TUI to start IPC server
4. Update checkpoint commands to send notifications
5. Remove chokidar from CheckpointPanel
6. Run full test suite
7. Manually test on Linux, Mac, Windows

## References

- [Node.js net module documentation](https://nodejs.org/api/net.html)
- [Unix domain sockets](https://en.wikipedia.org/wiki/Unix_domain_socket)
- [Windows named pipes](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes)
- [CVE-2022-23812: node-ipc vulnerability](https://github.com/advisories/GHSA-97m3-w2cp-4xx6)
