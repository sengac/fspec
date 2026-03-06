/**
 * MCP Streamable HTTP Server
 *
 * Implements the MCP Streamable HTTP transport:
 * - POST /mcp: JSON-RPC requests (initialize, tools/list, tools/call)
 * - GET /mcp: SSE stream for server-initiated notifications
 * - DELETE /mcp: Session termination
 *
 * Uses Node.js built-in http module only (zero dependencies).
 */

import { createServer } from 'http';
import { randomUUID } from 'crypto';
import { encodeNativeMessage, createNativeMessageReader } from './native-messaging.mjs';

const MCP_PROTOCOL_VERSION = '2025-03-26';
const SERVER_NAME = 'fspec-browser-agent';
const SERVER_VERSION = '0.1.0';

/** Default native browser control tools */
const NATIVE_TOOLS = [
  {
    name: 'browser_navigate',
    description: 'Navigate the active browser tab to a URL',
    inputSchema: {
      type: 'object',
      properties: {
        url: { type: 'string', description: 'The URL to navigate to' },
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
      },
      required: ['url'],
    },
  },
  {
    name: 'browser_screenshot',
    description: 'Capture a screenshot of a browser tab',
    inputSchema: {
      type: 'object',
      properties: {
        tabId: { type: 'number', description: 'Tab ID to capture' },
        fullPage: { type: 'boolean', description: 'Capture full scrollable page' },
      },
    },
  },
  {
    name: 'browser_list_tabs',
    description: 'List all open browser tabs',
    inputSchema: {
      type: 'object',
      properties: {},
    },
  },
  {
    name: 'browser_execute_script',
    description: 'Execute JavaScript in a browser tab',
    inputSchema: {
      type: 'object',
      properties: {
        code: { type: 'string', description: 'JavaScript code to execute' },
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
      },
      required: ['code'],
    },
  },
  {
    name: 'browser_switch_tab',
    description: 'Switch to a specific browser tab by activating it and focusing its window',
    inputSchema: {
      type: 'object',
      properties: {
        tabId: { type: 'number', description: 'The tab ID to switch to' },
      },
      required: ['tabId'],
    },
  },
  {
    name: 'browser_close_tab',
    description: 'Close a browser tab',
    inputSchema: {
      type: 'object',
      properties: {
        tabId: { type: 'number', description: 'The tab ID to close' },
      },
      required: ['tabId'],
    },
  },
  {
    name: 'browser_get_page_content',
    description: 'Get the content of a browser tab as text or HTML',
    inputSchema: {
      type: 'object',
      properties: {
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
        format: { type: 'string', enum: ['text', 'html'], description: 'Content format: "text" for innerText, "html" for outerHTML (defaults to "text")' },
      },
    },
  },
  {
    name: 'browser_click_element',
    description: 'Click an element on the page by CSS selector',
    inputSchema: {
      type: 'object',
      properties: {
        selector: { type: 'string', description: 'CSS selector of the element to click' },
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
      },
      required: ['selector'],
    },
  },
  {
    name: 'browser_fill_form',
    description: 'Fill a form field on the page by CSS selector',
    inputSchema: {
      type: 'object',
      properties: {
        selector: { type: 'string', description: 'CSS selector of the input element' },
        value: { type: 'string', description: 'Value to set on the input element' },
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
      },
      required: ['selector', 'value'],
    },
  },
  {
    name: 'browser_go_back',
    description: 'Navigate the browser tab back in history',
    inputSchema: {
      type: 'object',
      properties: {
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
      },
    },
  },
  {
    name: 'browser_go_forward',
    description: 'Navigate the browser tab forward in history',
    inputSchema: {
      type: 'object',
      properties: {
        tabId: { type: 'number', description: 'Optional tab ID (defaults to active tab)' },
      },
    },
  },
  {
    name: 'browser_create_tab',
    description: 'Create a new browser tab, optionally navigating to a URL',
    inputSchema: {
      type: 'object',
      properties: {
        url: { type: 'string', description: 'URL to open (defaults to New Tab page)' },
        active: { type: 'boolean', description: 'Whether to make it the active tab (defaults to true)' },
        windowId: { type: 'number', description: 'Window to create the tab in (defaults to current window)' },
        pinned: { type: 'boolean', description: 'Whether to pin the tab (defaults to false)' },
      },
    },
  },
];

/**
 * Create an MCP Streamable HTTP server.
 *
 * @param {object} options
 * @param {number} options.port - Port to listen on (0 for ephemeral)
 * @param {import('stream').Readable} [options.stdin] - Stdin stream (for native messaging)
 * @param {import('stream').Writable} [options.stdout] - Stdout stream (for native messaging)
 * @returns {{ start: () => Promise<number>, stop: () => Promise<void> }}
 */
export function createMcpServer({ port = 19876, stdin = null, stdout = null } = {}) {
  /** @type {Map<string, { sseResponses: Set<import('http').ServerResponse>, tools: Array<object> }>} */
  const sessions = new Map();

  /** @type {Map<string, { resolve: (value: object) => void, timer: ReturnType<typeof setTimeout> }>} */
  const pendingCalls = new Map();

  /** Latest WebMCP tools from the extension — used to seed new sessions */
  let latestWebmcpTools = [];

  // Set up native messaging reader if stdin provided
  let nativeReader = null;
  if (stdin) {
    nativeReader = createNativeMessageReader(stdin, (message) => {
      handleNativeMessage(message);
    });
  }

  function handleNativeMessage(message) {
    // Handle correlation-based responses
    if (message.correlationId && pendingCalls.has(message.correlationId)) {
      const pending = pendingCalls.get(message.correlationId);
      clearTimeout(pending.timer);
      pendingCalls.delete(message.correlationId);
      // Preserve the full message so callers can distinguish result vs error
      pending.resolve({ result: message.result, error: message.error });
      return;
    }

    // Handle notifications from extension
    if (message.type === 'NOTIFICATION' && message.notification) {
      // Broadcast to all sessions' SSE streams
      for (const [, session] of sessions) {
        for (const res of session.sseResponses) {
          const sseData = `data: ${JSON.stringify(message.notification)}\n\n`;
          res.write(sseData);
        }
      }
      return;
    }

    // Handle tool registry updates — update internal state AND notify agents via SSE
    if (message.type === 'TOOLS_CHANGED' && message.tools) {
      latestWebmcpTools = message.tools;
      for (const [, session] of sessions) {
        session.tools = message.tools;
        // Broadcast notifications/tools/list_changed to all SSE streams
        const notification = {
          jsonrpc: '2.0',
          method: 'notifications/tools/list_changed',
        };
        for (const res of session.sseResponses) {
          const sseData = `data: ${JSON.stringify(notification)}\n\n`;
          res.write(sseData);
        }
      }
    }
  }

  /**
   * Send a tool call to the extension via native messaging and wait for response.
   * @param {string} method
   * @param {object} params
   * @returns {Promise<object>}
   */
  function callExtension(params) {
    return new Promise((resolve, reject) => {
      const correlationId = randomUUID();
      const timeout = 30000; // 30s timeout

      const timer = setTimeout(() => {
        pendingCalls.delete(correlationId);
        reject(new Error('Extension call timed out'));
      }, timeout);

      pendingCalls.set(correlationId, { resolve, timer });

      const frame = encodeNativeMessage({
        type: 'TOOL_CALL',
        correlationId,
        params,
      });

      if (stdout) {
        stdout.write(frame);
      } else {
        // No extension connected — resolve with an error
        clearTimeout(timer);
        pendingCalls.delete(correlationId);
        resolve({ error: { code: -1, message: 'No extension connected' } });
      }
    });
  }

  function readBody(req) {
    return new Promise((resolve, reject) => {
      let body = '';
      req.on('data', (chunk) => { body += chunk; });
      req.on('end', () => { resolve(body); });
      req.on('error', reject);
    });
  }

  function sendJson(res, status, data, extraHeaders = {}) {
    const body = JSON.stringify(data);
    res.writeHead(status, {
      'Content-Type': 'application/json',
      ...extraHeaders,
    });
    res.end(body);
  }

  function sendError(res, status, message) {
    res.writeHead(status, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: message }));
  }

  async function handlePost(req, res) {
    const body = await readBody(req);
    let request;

    try {
      request = JSON.parse(body);
    } catch {
      sendError(res, 400, 'Invalid JSON');
      return;
    }

    const { method, id, params } = request;

    // Initialize doesn't require a session ID
    if (method === 'initialize') {
      const sessionId = randomUUID();
      sessions.set(sessionId, {
        sseResponses: new Set(),
        tools: [...latestWebmcpTools],
      });

      sendJson(res, 200, {
        jsonrpc: '2.0',
        id,
        result: {
          protocolVersion: MCP_PROTOCOL_VERSION,
          serverInfo: { name: SERVER_NAME, version: SERVER_VERSION },
          capabilities: {
            tools: { listChanged: true },
          },
        },
      }, {
        'Mcp-Session-Id': sessionId,
      });
      return;
    }

    // All other methods require a valid session ID
    const sessionId = req.headers['mcp-session-id'];

    if (!sessionId) {
      sendError(res, 400, 'Missing Mcp-Session-Id header');
      return;
    }

    if (!sessions.has(sessionId)) {
      sendError(res, 404, 'Session not found');
      return;
    }

    if (method === 'tools/list') {
      const session = sessions.get(sessionId);
      const allTools = [...NATIVE_TOOLS, ...session.tools];
      sendJson(res, 200, {
        jsonrpc: '2.0',
        id,
        result: { tools: allTools },
      });
      return;
    }

    if (method === 'tools/call') {
      try {
        const response = await callExtension(params);
        if (response.error) {
          // Error from extension — propagate as JSON-RPC error
          const error = typeof response.error === 'object'
            ? response.error
            : { code: -1, message: String(response.error) };
          sendJson(res, 200, {
            jsonrpc: '2.0',
            id,
            error,
          });
        } else {
          sendJson(res, 200, {
            jsonrpc: '2.0',
            id,
            result: response.result || {},
          });
        }
      } catch (err) {
        sendJson(res, 200, {
          jsonrpc: '2.0',
          id,
          error: { code: -1, message: err.message },
        });
      }
      return;
    }

    // Unknown method
    sendJson(res, 200, {
      jsonrpc: '2.0',
      id,
      error: { code: -32601, message: `Method not found: ${method}` },
    });
  }

  function handleGet(req, res) {
    const accept = req.headers['accept'] || '';
    if (!accept.includes('text/event-stream')) {
      sendError(res, 400, 'Accept header must include text/event-stream');
      return;
    }

    const sessionId = req.headers['mcp-session-id'];
    if (!sessionId || !sessions.has(sessionId)) {
      sendError(res, 404, 'Session not found');
      return;
    }

    const session = sessions.get(sessionId);

    // Set up SSE response
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    });

    // Send initial comment to keep connection alive
    res.write(':ok\n\n');

    session.sseResponses.add(res);

    // Clean up on client disconnect
    req.on('close', () => {
      session.sseResponses.delete(res);
    });
  }

  function handleDelete(req, res) {
    const sessionId = req.headers['mcp-session-id'];
    if (!sessionId || !sessions.has(sessionId)) {
      sendError(res, 404, 'Session not found');
      return;
    }

    const session = sessions.get(sessionId);

    // Close all SSE streams for this session
    for (const sseRes of session.sseResponses) {
      sseRes.end();
    }
    session.sseResponses.clear();

    // Remove session
    sessions.delete(sessionId);

    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true }));
  }

  const server = createServer((req, res) => {
    // Only handle /mcp endpoint
    const url = new URL(req.url, `http://localhost`);
    if (url.pathname !== '/mcp') {
      sendError(res, 404, 'Not found');
      return;
    }

    // CORS headers for local development
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, DELETE, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Mcp-Session-Id, Accept');
    res.setHeader('Access-Control-Expose-Headers', 'Mcp-Session-Id');

    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      res.end();
      return;
    }

    switch (req.method) {
      case 'POST':
        handlePost(req, res);
        break;
      case 'GET':
        handleGet(req, res);
        break;
      case 'DELETE':
        handleDelete(req, res);
        break;
      default:
        sendError(res, 405, 'Method not allowed');
    }
  });

  return {
    start() {
      return new Promise((resolve) => {
        server.listen(port, '127.0.0.1', () => {
          const addr = server.address();
          resolve(addr.port);
        });
      });
    },
    stop() {
      return new Promise((resolve) => {
        // Clean up all sessions
        for (const [, session] of sessions) {
          for (const sseRes of session.sseResponses) {
            sseRes.end();
          }
        }
        sessions.clear();

        // Clean up pending calls
        for (const [, pending] of pendingCalls) {
          clearTimeout(pending.timer);
        }
        pendingCalls.clear();

        // Stop native message reader
        if (nativeReader) {
          nativeReader.stop();
        }

        server.close(() => {
          resolve();
        });
      });
    },
    /** Expose sessions for testing */
    get sessions() {
      return sessions;
    },
  };
}
