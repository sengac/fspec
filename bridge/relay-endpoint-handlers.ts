/**
 * Relay Endpoint Message Handlers
 *
 * Extracted from relay-endpoint.ts to keep file sizes under 300 lines.
 * Handles inbound relay messages and codelet session messages.
 *
 * BRIDGE-015: Platform-Agnostic Relay Bridge Endpoint
 */
import type {
  RelayEndpointState,
  RelayMessage,
  FspecInboundMessage,
  SessionCreateCallback,
} from './relay-types';

const LOG_PREFIX = '[relay-endpoint]';

// ---------------------------------------------------------------------------
// Relay → fspec translation
// ---------------------------------------------------------------------------

/**
 * Translate relay input message to fspec flat InboundMessage format.
 * Unwraps the data envelope so message and images are top-level fields.
 */
export function translateRelayInputToFspec(
  relayMsg: RelayMessage
): FspecInboundMessage {
  const data = relayMsg.data || {};
  return {
    type: 'input',
    session_id: relayMsg.session_id || '',
    message: data.message as string | undefined,
    images: data.images as
      | Array<{ data: string; media_type: string }>
      | undefined,
  };
}

/**
 * Translate relay sessionControl to fspec control format.
 * Renames type from 'sessionControl' to 'control' and unwraps data envelope.
 */
export function translateRelaySessionControlToFspec(
  relayMsg: RelayMessage
): FspecInboundMessage {
  const data = relayMsg.data || {};
  return {
    type: 'control',
    session_id: relayMsg.session_id || '',
    action: data.action as string | undefined,
    response: data.response as string | undefined,
  };
}

/**
 * Translate relay command message to fspec flat InboundMessage format.
 * Unwraps the data envelope and serialises args to args_json string.
 */
export function translateRelayCommandToFspec(
  relayMsg: RelayMessage
): FspecInboundMessage {
  const data = relayMsg.data || {};
  return {
    type: 'command',
    session_id: relayMsg.session_id || '',
    message: '',
    request_id: relayMsg.request_id || '',
    command: data.command as string,
    args_json: JSON.stringify(data.args || {}),
  };
}

// ---------------------------------------------------------------------------
// Context for message handlers
// ---------------------------------------------------------------------------

export interface HandlerContext {
  state: RelayEndpointState;
  sendToRelay: (msg: Record<string, unknown>) => void;
  startLocalServer: () => void;
  startPingInterval: () => void;
  onSessionCreate?: SessionCreateCallback;
}

/** Minimal WebSocket interface for codelet connections */
export interface CodeletSocket {
  send: (data: string) => void;
}

// ---------------------------------------------------------------------------
// Relay session lookup helper
// ---------------------------------------------------------------------------

function getSessionOrWarn(
  state: RelayEndpointState,
  sessionId: string | undefined,
  context: string
): CodeletSocket | null {
  const id = sessionId || '';
  const ws = state.sessions.get(id);
  if (!ws) {
    console.warn(
      `${LOG_PREFIX} No codelet connection for session ${id}${context ? ` (${context})` : ''}`
    );
  }
  return ws ?? null;
}

// ---------------------------------------------------------------------------
// Session create handler
// ---------------------------------------------------------------------------

function handleSessionCreate(msg: RelayMessage, ctx: HandlerContext): void {
  if (!ctx.onSessionCreate) {
    console.warn(
      `${LOG_PREFIX} session:create received but no onSessionCreate callback registered`
    );
    if (msg.request_id) {
      ctx.sendToRelay({
        type: 'session:create:error',
        request_id: msg.request_id,
        data: {
          error: 'SESSION_CREATE_NOT_SUPPORTED',
          message: 'This instance does not support remote session creation',
        },
      });
    }
    return;
  }

  const requestId = msg.request_id;
  console.log(
    `${LOG_PREFIX} session:create request received (request_id: ${requestId ?? 'none'})`
  );

  void (async () => {
    try {
      const result = await ctx.onSessionCreate!();
      console.log(`${LOG_PREFIX} Session created: ${result.session_id}`);

      if (requestId) {
        ctx.sendToRelay({
          type: 'session:created',
          request_id: requestId,
          data: { session_id: result.session_id },
        });
      }

      const sessionIds = Array.from(ctx.state.sessions.keys());
      if (!sessionIds.includes(result.session_id)) {
        sessionIds.push(result.session_id);
      }
      ctx.sendToRelay({
        type: 'metadataUpdate',
        data: {
          sessions: sessionIds.map(id => ({
            id,
            state: id === result.session_id ? 'idle' : 'running',
          })),
        },
      });
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      console.error(`${LOG_PREFIX} session:create failed:`, errMsg);

      if (requestId) {
        ctx.sendToRelay({
          type: 'session:create:error',
          request_id: requestId,
          data: {
            error: 'SESSION_CREATE_FAILED',
            message: errMsg,
          },
        });
      }
    }
  })();
}

// ---------------------------------------------------------------------------
// Relay message handler
// ---------------------------------------------------------------------------

export function handleRelayMessage(data: string, ctx: HandlerContext): void {
  let msg: RelayMessage;
  try {
    msg = JSON.parse(data) as RelayMessage;
  } catch {
    console.error(`${LOG_PREFIX} Failed to parse relay message`);
    return;
  }

  switch (msg.type) {
    case 'authSuccess':
      ctx.state.authenticated = true;
      ctx.state.authError = undefined;
      console.log(`${LOG_PREFIX} Authenticated with relay`);
      ctx.startLocalServer();
      ctx.startPingInterval();
      break;

    case 'authError': {
      ctx.state.authenticated = false;
      const errorData = msg.data || {};
      ctx.state.authError = {
        code: (errorData.code as string) || 'UNKNOWN',
        message: (errorData.message as string) || 'Authentication failed',
      };
      console.error(
        `${LOG_PREFIX} Authentication failed`,
        ctx.state.authError.code
      );
      break;
    }

    case 'pong':
      ctx.state.connectionAlive = true;
      break;

    case 'input': {
      const sessionWs = getSessionOrWarn(ctx.state, msg.session_id, '');
      if (!sessionWs) {
        return;
      }
      sessionWs.send(JSON.stringify(translateRelayInputToFspec(msg)));
      break;
    }

    case 'sessionControl': {
      const sessionWs = getSessionOrWarn(ctx.state, msg.session_id, '');
      if (!sessionWs) {
        return;
      }
      sessionWs.send(JSON.stringify(translateRelaySessionControlToFspec(msg)));
      break;
    }

    case 'command': {
      const sessionWs = getSessionOrWarn(ctx.state, msg.session_id, 'command');
      if (!sessionWs) {
        return;
      }
      sessionWs.send(JSON.stringify(translateRelayCommandToFspec(msg)));
      break;
    }

    case 'session:create':
      handleSessionCreate(msg, ctx);
      break;

    default:
      break;
  }
}

// ---------------------------------------------------------------------------
// Codelet message handler
// ---------------------------------------------------------------------------

export function handleCodeletMessage(
  ws: CodeletSocket,
  data: string,
  ctx: HandlerContext
): void {
  let msg: Record<string, unknown>;
  try {
    msg = JSON.parse(data) as Record<string, unknown>;
  } catch {
    console.error(`${LOG_PREFIX} Failed to parse codelet message`);
    return;
  }

  const msgType = msg.type as string;

  if (msgType === 'connected') {
    const sessionId = msg.session_id as string;
    ctx.state.sessions.set(sessionId, ws);
    console.log(`${LOG_PREFIX} Session registered: ${sessionId}`);
    ctx.sendToRelay(msg);
    return;
  }

  // chunk + any other messages pass through to relay
  ctx.sendToRelay(msg);
}
