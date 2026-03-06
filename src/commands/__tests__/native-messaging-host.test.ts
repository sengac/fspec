/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the acceptance criteria for EXT-003:
 * Native Messaging Host & MCP Streamable HTTP Server.
 * Scenarios map directly to Gherkin scenarios tagged @EXT-003.
 */

import { describe, it, expect } from 'vitest';
import http, { type IncomingMessage } from 'http';
import { resolve } from 'path';
import { existsSync, readFileSync, mkdirSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { randomUUID } from 'crypto';
import { PassThrough } from 'stream';

const PROJECT_ROOT = resolve(import.meta.dirname, '..', '..', '..');
const HOST_DIR = resolve(PROJECT_ROOT, 'extension', 'host');
const HOST_LIB_DIR = resolve(HOST_DIR, 'lib');

// Lazily-cached dynamic imports for the .mjs host modules
let _mcpServerModule: {
  createMcpServer: (opts?: Record<string, unknown>) => Record<string, unknown>;
} | null = null;
let _nativeMessagingModule: {
  encodeNativeMessage: (msg: Record<string, unknown>) => Buffer;
  decodeNativeMessage: (buf: Buffer) => Record<string, unknown>;
} | null = null;
let _registrationModule: {
  registerNativeHost: (
    opts: Record<string, unknown>
  ) => Promise<{ manifestPath: string; manifest: Record<string, unknown> }>;
} | null = null;

async function getMcpServerModule() {
  if (!_mcpServerModule) {
    _mcpServerModule = await import(
      /* @vite-ignore */ resolve(HOST_LIB_DIR, 'mcp-server.mjs')
    );
  }
  return _mcpServerModule;
}

async function getNativeMessagingModule() {
  if (!_nativeMessagingModule) {
    _nativeMessagingModule = await import(
      /* @vite-ignore */ resolve(HOST_LIB_DIR, 'native-messaging.mjs')
    );
  }
  return _nativeMessagingModule;
}

async function getRegistrationModule() {
  if (!_registrationModule) {
    _registrationModule = await import(
      /* @vite-ignore */ resolve(HOST_LIB_DIR, 'registration.mjs')
    );
  }
  return _registrationModule;
}

/**
 * Helper: make an HTTP request and return the response
 */
function httpRequest(
  port: number,
  method: string,
  path: string,
  headers: Record<string, string> = {},
  body?: string
): Promise<{ status: number; headers: Record<string, string>; body: string }> {
  return new Promise((resolvePromise, reject) => {
    const req = http.request(
      { hostname: '127.0.0.1', port, path, method, headers },
      (res: IncomingMessage) => {
        let data = '';
        res.on('data', (chunk: Buffer) => {
          data += chunk.toString();
        });
        res.on('end', () => {
          const responseHeaders: Record<string, string> = {};
          for (const [key, value] of Object.entries(res.headers)) {
            if (typeof value === 'string') {
              responseHeaders[key] = value;
            }
          }
          resolvePromise({
            status: res.statusCode ?? 0,
            headers: responseHeaders,
            body: data,
          });
        });
      }
    );
    req.on('error', reject);
    if (body) {
      req.write(body);
    }
    req.end();
  });
}

/**
 * Helper: open an SSE stream and collect events
 */
function openSSEStream(
  port: number,
  sessionId: string
): Promise<{ response: IncomingMessage; events: string[]; close: () => void }> {
  return new Promise((resolvePromise, reject) => {
    const req = http.request(
      {
        hostname: '127.0.0.1',
        port,
        path: '/mcp',
        method: 'GET',
        headers: {
          Accept: 'text/event-stream',
          'Mcp-Session-Id': sessionId,
        },
      },
      (res: IncomingMessage) => {
        const events: string[] = [];
        res.on('data', (chunk: Buffer) => {
          events.push(chunk.toString());
        });
        resolvePromise({
          response: res,
          events,
          close: () => {
            req.destroy();
          },
        });
      }
    );
    req.on('error', reject);
    req.end();
  });
}

/**
 * Helper: send MCP initialize request and return session ID + response
 */
async function initializeSession(port: number): Promise<{
  sessionId: string;
  response: { status: number; headers: Record<string, string>; body: string };
}> {
  const initRequest = {
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'test-agent', version: '1.0.0' },
    },
  };
  const response = await httpRequest(
    port,
    'POST',
    '/mcp',
    { 'Content-Type': 'application/json' },
    JSON.stringify(initRequest)
  );
  const sessionId = response.headers['mcp-session-id'];
  return { sessionId, response };
}

/**
 * Helper: create an MCP server, start it, and return a cleanup handle.
 * Accepts optional stdin/stdout streams for native messaging tests.
 */
async function startTestServer(
  options: { stdin?: PassThrough; stdout?: PassThrough } = {}
): Promise<{
  port: number;
  stop: () => Promise<void>;
}> {
  const { createMcpServer } = await getMcpServerModule();
  const server = createMcpServer({ port: 0, ...options }) as {
    start: () => Promise<number>;
    stop: () => Promise<void>;
  };
  const port = await server.start();
  return { port, stop: () => server.stop() };
}

describe('Feature: fspec Browser Agent Chrome Extension', () => {
  describe('Scenario: Connect to extension and discover available tools', () => {
    it('should respond to MCP initialize with server info, capabilities, and session ID', async () => {
      // @step Given the fspec WebMCP Chrome extension is installed and running
      // Extension installation is assumed for integration context

      // @step And the native messaging host is listening on port 19876
      const server = await startTestServer();

      try {
        // @step When the agent calls ConnectMCP with transport "http" and url "http://localhost:19876/mcp"
        const { sessionId, response } = await initializeSession(server.port);

        // @step Then the extension responds with a successful MCP initialize handshake
        expect(response.status).toBe(200);
        const result = JSON.parse(response.body);
        expect(result.jsonrpc).toBe('2.0');
        expect(result.id).toBe(1);
        expect(result.result).toBeDefined();
        expect(result.result.protocolVersion).toBeDefined();
        expect(result.result.serverInfo).toBeDefined();
        expect(result.result.capabilities).toBeDefined();

        // @step And tools/list returns native browser control tools including "browser_navigate", "browser_screenshot", and "browser_list_tabs"
        const toolsResponse = await httpRequest(
          server.port,
          'POST',
          '/mcp',
          { 'Content-Type': 'application/json', 'Mcp-Session-Id': sessionId },
          JSON.stringify({
            jsonrpc: '2.0',
            id: 2,
            method: 'tools/list',
            params: {},
          })
        );
        const toolsResult = JSON.parse(toolsResponse.body);
        const toolNames = toolsResult.result.tools.map(
          (t: { name: string }) => t.name
        );
        expect(toolNames).toContain('browser_navigate');
        expect(toolNames).toContain('browser_screenshot');
        expect(toolNames).toContain('browser_list_tabs');

        // @step And the agent receives an Mcp-Session-Id header for subsequent requests
        expect(sessionId).toBeDefined();
        expect(typeof sessionId).toBe('string');
        expect(sessionId.length).toBeGreaterThan(0);
      } finally {
        await server.stop();
      }
    });
  });

  describe('Scenario: Connection lifecycle with SSE stream and session termination', () => {
    it('should close SSE stream and clean up session on DELETE /mcp', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const server = await startTestServer();

      try {
        const { sessionId } = await initializeSession(server.port);
        expect(sessionId).toBeDefined();

        // @step And a GET /mcp SSE stream is open for server-to-client notifications
        const sse = await openSSEStream(server.port, sessionId);
        expect(sse.response.statusCode).toBe(200);
        expect(sse.response.headers['content-type']).toContain(
          'text/event-stream'
        );

        // @step When the agent disconnects via ConnectMCP
        // @step Then a DELETE /mcp request is sent to terminate the session
        const deleteResponse = await httpRequest(
          server.port,
          'DELETE',
          '/mcp',
          {
            'Mcp-Session-Id': sessionId,
          }
        );
        expect(deleteResponse.status).toBe(200);

        // @step And the SSE stream closes
        await new Promise<void>(resolvePromise => {
          sse.response.on('close', () => {
            resolvePromise();
          });
          // Give server time to close the stream
          setTimeout(() => {
            sse.close();
            resolvePromise();
          }, 500);
        });

        // @step And all mcp__ext__ tools are removed from the agent's tool list
        // Verify session no longer exists - subsequent request should 404
        const postDeleteResponse = await httpRequest(
          server.port,
          'POST',
          '/mcp',
          { 'Content-Type': 'application/json', 'Mcp-Session-Id': sessionId },
          JSON.stringify({
            jsonrpc: '2.0',
            id: 3,
            method: 'tools/list',
            params: {},
          })
        );
        expect(postDeleteResponse.status).toBe(404);
      } finally {
        await server.stop();
      }
    });
  });

  describe('Scenario: Native messaging host reads and writes Chrome native messaging frames', () => {
    it('should encode and decode messages using 4-byte little-endian length prefix protocol', async () => {
      // @step Given the native messaging host process is running
      const { encodeNativeMessage, decodeNativeMessage } =
        await getNativeMessagingModule();

      // @step When the Chrome extension sends a JSON message via stdin with a 4-byte little-endian length prefix
      const testMessage = {
        type: 'TOOLS_CHANGED',
        tools: [{ name: 'browser_navigate' }],
      };
      const encoded = encodeNativeMessage(
        testMessage as unknown as Record<string, unknown>
      );

      // @step Then the host reads the length prefix and parses the JSON payload
      expect(encoded).toBeInstanceOf(Buffer);
      const expectedJsonBytes = Buffer.from(
        JSON.stringify(testMessage),
        'utf-8'
      );
      const lengthPrefix = encoded.readUInt32LE(0);
      expect(lengthPrefix).toBe(expectedJsonBytes.length);
      const jsonBytes = encoded.subarray(4);
      expect(jsonBytes.toString('utf-8')).toBe(JSON.stringify(testMessage));

      const decoded = decodeNativeMessage(encoded);
      expect(decoded).toEqual(testMessage);

      // @step And the host can write responses to stdout using the same 4-byte length prefix format
      const responseMessage = {
        type: 'TOOL_RESULT',
        correlationId: 'abc-123',
        result: { success: true },
      };
      const encodedResponse = encodeNativeMessage(
        responseMessage as unknown as Record<string, unknown>
      );
      const decodedResponse = decodeNativeMessage(encodedResponse);
      expect(decodedResponse).toEqual(responseMessage);
    });
  });

  describe('Scenario: Route tool call from MCP client through native messaging to extension', () => {
    it('should forward tool calls to extension via stdout and return results from stdin', async () => {
      // @step Given the native messaging host is listening on port 19876
      const mockStdin = new PassThrough();
      const mockStdout = new PassThrough();
      const server = await startTestServer({
        stdin: mockStdin,
        stdout: mockStdout,
      });

      try {
        const { sessionId } = await initializeSession(server.port);

        // @step And a Chrome extension is connected via native messaging stdin/stdout
        // Mock extension is connected via mockStdin/mockStdout

        // @step When the agent sends a POST /mcp request with a tools/call JSON-RPC message
        const toolCallPromise = httpRequest(
          server.port,
          'POST',
          '/mcp',
          { 'Content-Type': 'application/json', 'Mcp-Session-Id': sessionId },
          JSON.stringify({
            jsonrpc: '2.0',
            id: 2,
            method: 'tools/call',
            params: {
              name: 'browser_navigate',
              arguments: { url: 'https://example.com' },
            },
          })
        );

        // @step Then the host writes a native messaging frame to stdout with a correlation ID
        const stdoutData = await new Promise<Buffer>(resolvePromise => {
          mockStdout.once('data', (chunk: Buffer) => {
            resolvePromise(chunk);
          });
        });
        const { decodeNativeMessage, encodeNativeMessage } =
          await getNativeMessagingModule();
        const forwardedMessage = decodeNativeMessage(stdoutData);
        expect(forwardedMessage.type).toBe('TOOL_CALL');
        expect(forwardedMessage.correlationId).toBeDefined();
        expect((forwardedMessage.params as Record<string, unknown>).name).toBe(
          'browser_navigate'
        );

        // @step And the host holds the HTTP response open until the extension replies on stdin
        const extensionResponse = encodeNativeMessage({
          correlationId: forwardedMessage.correlationId as string,
          result: {
            content: [
              { type: 'text', text: 'Navigated to https://example.com' },
            ],
          },
        } as unknown as Record<string, unknown>);
        mockStdin.write(extensionResponse);

        // @step And the host returns the extension's response as a JSON-RPC result to the agent
        const toolCallResponse = await toolCallPromise;
        expect(toolCallResponse.status).toBe(200);
        const result = JSON.parse(toolCallResponse.body);
        expect(result.jsonrpc).toBe('2.0');
        expect(result.id).toBe(2);
        expect(result.result).toBeDefined();
        expect(result.result.content).toBeDefined();
        expect(result.result.content[0].text).toContain('example.com');
      } finally {
        await server.stop();
      }
    });

    it('should return JSON-RPC error when extension replies with an error via stdin', async () => {
      // @step Given the native messaging host is listening on port 19876
      const mockStdin = new PassThrough();
      const mockStdout = new PassThrough();
      const server = await startTestServer({
        stdin: mockStdin,
        stdout: mockStdout,
      });

      try {
        const { sessionId } = await initializeSession(server.port);

        // @step And a Chrome extension is connected via native messaging stdin/stdout

        // @step When the agent sends a POST /mcp request with a tools/call JSON-RPC message
        const toolCallPromise = httpRequest(
          server.port,
          'POST',
          '/mcp',
          { 'Content-Type': 'application/json', 'Mcp-Session-Id': sessionId },
          JSON.stringify({
            jsonrpc: '2.0',
            id: 3,
            method: 'tools/call',
            params: {
              name: 'browser_navigate',
              arguments: { url: 'https://example.com' },
            },
          })
        );

        // @step Then the host writes a native messaging frame to stdout with a correlation ID
        const stdoutData = await new Promise<Buffer>(resolvePromise => {
          mockStdout.once('data', (chunk: Buffer) => {
            resolvePromise(chunk);
          });
        });
        const { decodeNativeMessage, encodeNativeMessage } =
          await getNativeMessagingModule();
        const forwardedMessage = decodeNativeMessage(stdoutData);
        expect(forwardedMessage.correlationId).toBeDefined();

        // @step And the extension replies with an error on stdin
        const extensionErrorResponse = encodeNativeMessage({
          correlationId: forwardedMessage.correlationId as string,
          error: { code: -1, message: 'Network timeout' },
        } as unknown as Record<string, unknown>);
        mockStdin.write(extensionErrorResponse);

        // @step And the host returns the extension's error as a JSON-RPC error to the agent
        const toolCallResponse = await toolCallPromise;
        expect(toolCallResponse.status).toBe(200);
        const parsed = JSON.parse(toolCallResponse.body);
        expect(parsed.jsonrpc).toBe('2.0');
        expect(parsed.id).toBe(3);
        expect(parsed.error).toBeDefined();
        expect(parsed.error.code).toBe(-1);
        expect(parsed.error.message).toBe('Network timeout');
        // Must NOT have a result field
        expect(parsed.result).toBeUndefined();
      } finally {
        await server.stop();
      }
    });
  });

  describe('Scenario: SSE notification stream delivers extension events to agent', () => {
    it('should deliver extension events as SSE data lines on GET /mcp stream', async () => {
      // @step Given the agent has an active MCP session with a valid Mcp-Session-Id
      const mockStdin = new PassThrough();
      const mockStdout = new PassThrough();
      const server = await startTestServer({
        stdin: mockStdin,
        stdout: mockStdout,
      });

      try {
        const { sessionId } = await initializeSession(server.port);

        // @step When the agent opens a GET /mcp request with Accept header "text/event-stream"
        const sse = await openSSEStream(server.port, sessionId);

        // @step Then the server responds with status 200 and Content-Type "text/event-stream"
        expect(sse.response.statusCode).toBe(200);
        expect(sse.response.headers['content-type']).toContain(
          'text/event-stream'
        );

        // @step And the SSE stream stays open for the duration of the session
        let streamEnded = false;
        sse.response.on('end', () => {
          streamEnded = true;
        });

        // @step And when the extension sends a browser event via stdin the host writes it as an SSE data line
        const { encodeNativeMessage } = await getNativeMessagingModule();
        const browserEvent = encodeNativeMessage({
          type: 'NOTIFICATION',
          notification: {
            jsonrpc: '2.0',
            method: 'notifications/browser/navigation',
            params: {
              tabId: 123,
              url: 'https://new-page.com',
              title: 'New Page',
            },
          },
        } as unknown as Record<string, unknown>);
        mockStdin.write(browserEvent);

        // Wait for the SSE event to arrive
        await new Promise(r => setTimeout(r, 200));

        // Check that we received an SSE event
        const allEventData = sse.events.join('');
        expect(allEventData).toContain('data:');
        expect(allEventData).toContain('notifications/browser/navigation');
        expect(streamEnded).toBe(false);

        sse.close();
      } finally {
        await server.stop();
      }
    });

    it('should broadcast notifications/tools/list_changed via SSE when TOOLS_CHANGED arrives from extension', async () => {
      // @step Given the agent has an active MCP session with SSE stream open
      const mockStdin = new PassThrough();
      const mockStdout = new PassThrough();
      const server = await startTestServer({
        stdin: mockStdin,
        stdout: mockStdout,
      });

      try {
        const { sessionId } = await initializeSession(server.port);
        const sse = await openSSEStream(server.port, sessionId);

        // @step When the extension sends a TOOLS_CHANGED message with updated tool list
        const { encodeNativeMessage } = await getNativeMessagingModule();
        const toolsChangedMsg = encodeNativeMessage({
          type: 'TOOLS_CHANGED',
          tools: [
            {
              name: 'webmcp__example.com__searchFlights',
              description: 'Search for flights',
              source: 'webmcp',
              origin: 'example.com',
              tabId: 42,
            },
          ],
        } as unknown as Record<string, unknown>);
        mockStdin.write(toolsChangedMsg);

        // Wait for the SSE event to arrive
        await new Promise(r => setTimeout(r, 200));

        // @step Then the SSE stream receives a notifications/tools/list_changed event
        const allEventData = sse.events.join('');
        expect(allEventData).toContain('data:');
        expect(allEventData).toContain('notifications/tools/list_changed');

        sse.close();
      } finally {
        await server.stop();
      }
    });
  });

  describe('Scenario: Register native messaging host with Chrome', () => {
    it('should write a Chrome native messaging host manifest to the correct platform directory', async () => {
      // @step Given the native messaging host script exists at extension/host/native-host.js
      const { registerNativeHost } = await getRegistrationModule();

      // Use a temp directory to avoid writing to actual Chrome config
      const testDir = resolve(tmpdir(), `fspec-test-${randomUUID()}`);
      mkdirSync(testDir, { recursive: true });

      try {
        // @step When the user runs the host with "--register" flag and "--extension-id" with a valid Chrome extension ID
        const extensionId = 'abcdefghijklmnopabcdefghijklmnop';
        const hostScriptPath = resolve(HOST_DIR, 'native-host.mjs');
        await registerNativeHost({
          extensionId,
          hostScriptPath,
          outputDir: testDir, // Override for testing
        });

        // @step Then the host writes a com.fspec.browser.agent.json manifest to the platform-specific Chrome NativeMessagingHosts directory
        const manifestPath = resolve(testDir, 'com.fspec.browser.agent.json');
        expect(existsSync(manifestPath)).toBe(true);
        const manifest = JSON.parse(readFileSync(manifestPath, 'utf-8'));

        // @step And the manifest contains the correct host name "com.fspec.browser.agent"
        expect(manifest.name).toBe('com.fspec.browser.agent');

        // @step And the manifest contains the absolute path to the host script
        expect(manifest.path).toBeDefined();
        expect(typeof manifest.path).toBe('string');
        expect(manifest.path).toContain('native-host');

        // @step And the manifest contains the extension ID in allowed_origins
        expect(manifest.allowed_origins).toBeDefined();
        expect(Array.isArray(manifest.allowed_origins)).toBe(true);
        expect(manifest.allowed_origins[0]).toContain(extensionId);
      } finally {
        rmSync(testDir, { recursive: true, force: true });
      }
    });
  });

  describe('Scenario: Reject requests with missing session ID', () => {
    it('should respond 400 when Mcp-Session-Id is missing for non-initialize request', async () => {
      // @step Given the native messaging host MCP server is running
      const server = await startTestServer();

      try {
        // @step And a session has been initialized with a valid Mcp-Session-Id
        const { response: initResponse } = await initializeSession(server.port);
        expect(initResponse.status).toBe(200);

        // @step When a POST /mcp request arrives without an Mcp-Session-Id header for a non-initialize method
        const response = await httpRequest(
          server.port,
          'POST',
          '/mcp',
          { 'Content-Type': 'application/json' },
          JSON.stringify({
            jsonrpc: '2.0',
            id: 2,
            method: 'tools/list',
            params: {},
          })
        );

        // @step Then the server responds with status 400 Bad Request
        expect(response.status).toBe(400);
      } finally {
        await server.stop();
      }
    });
  });

  describe('Scenario: Reject requests with invalid session ID', () => {
    it('should respond 404 when Mcp-Session-Id does not match any active session', async () => {
      // @step Given the native messaging host MCP server is running
      const server = await startTestServer();

      try {
        // @step When a POST /mcp request arrives with an Mcp-Session-Id that does not match any active session
        const response = await httpRequest(
          server.port,
          'POST',
          '/mcp',
          {
            'Content-Type': 'application/json',
            'Mcp-Session-Id': 'nonexistent-session-id-12345',
          },
          JSON.stringify({
            jsonrpc: '2.0',
            id: 1,
            method: 'tools/list',
            params: {},
          })
        );

        // @step Then the server responds with status 404 Not Found
        expect(response.status).toBe(404);
      } finally {
        await server.stop();
      }
    });
  });
});
