#!/usr/bin/env node

/**
 * fspec WebMCP Native Messaging Host
 *
 * Entry point for the Chrome native messaging host.
 * Chrome launches this script via chrome.runtime.connectNative('com.fspec.webmcp').
 *
 * Responsibilities:
 * - Run a Streamable HTTP MCP server on port 19876 (configurable)
 * - Communicate with Chrome extension via stdin/stdout native messaging protocol
 * - Relay MCP tool calls to extension and return results
 * - Forward extension notifications to connected agents via SSE
 *
 * Usage:
 *   node native-host.mjs                            # Start the host
 *   node native-host.mjs --port 8080                # Custom port
 *   node native-host.mjs --register --extension-id <id>  # Register with Chrome
 */

import { createMcpServer } from './lib/mcp-server.mjs';
import { registerNativeHost } from './lib/registration.mjs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function parseArgs(argv) {
  const args = { port: 19876 };

  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === '--register') {
      args.register = true;
    } else if (argv[i] === '--extension-id' && argv[i + 1]) {
      args.extensionId = argv[++i];
    } else if (argv[i] === '--port' && argv[i + 1]) {
      args.port = parseInt(argv[++i], 10);
    }
  }

  return args;
}

async function main() {
  const args = parseArgs(process.argv);

  if (args.register) {
    if (!args.extensionId) {
      process.stderr.write('Error: --extension-id is required with --register\n');
      process.exit(1);
    }

    const hostScriptPath = resolve(__dirname, 'native-host.mjs');
    const { manifestPath } = await registerNativeHost({
      extensionId: args.extensionId,
      hostScriptPath,
    });

    process.stderr.write(`✓ Registered native messaging host at: ${manifestPath}\n`);
    process.exit(0);
  }

  // Start MCP server
  const server = createMcpServer({
    port: args.port,
    stdin: process.stdin,
    stdout: process.stdout,
  });

  const actualPort = await server.start();
  process.stderr.write(`fspec WebMCP native host listening on port ${actualPort}\n`);

  // Handle graceful shutdown
  process.on('SIGINT', async () => {
    await server.stop();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await server.stop();
    process.exit(0);
  });

  // Handle stdin close (Chrome disconnects)
  process.stdin.on('end', async () => {
    await server.stop();
    process.exit(0);
  });
}

main();
