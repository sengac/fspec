/**
 * Relay Server CLI Entry Point
 *
 * Starts the relay server as a standalone process.
 * Run via: npm run bridge:server
 *
 * BRIDGE-019: Relay Server
 */

import { config } from 'dotenv';
import { startRelayServer } from './relay-server';
import type { RelayServerConfig } from './relay-server';

const LOG_PREFIX = '[relay-server]';

config();

const port = parseInt(process.env.RELAY_SERVER_PORT || '8765', 10);
const apiKey = process.env.RELAY_SERVER_API_KEY || undefined;

const serverConfig: RelayServerConfig = { port, apiKey };
const { wss } = startRelayServer(serverConfig);

process.on('SIGINT', () => {
  console.log(`\n${LOG_PREFIX} Shutting down...`);
  wss.close();
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log(`${LOG_PREFIX} Received SIGTERM, shutting down...`);
  wss.close();
  process.exit(0);
});
