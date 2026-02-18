/**
 * Telegram Bridge Endpoint
 *
 * Standalone WebSocket server that bridges codelet sessions to Telegram.
 * This module exports testable functions and can be run as a standalone process.
 *
 * BRIDGE-002: Telegram Bridge Endpoint
 *
 * Architecture:
 * - WebSocket server (ws) listens for codelet BridgeManager connections
 * - Telegram Bot API (node-telegram-bot-api) with polling mode
 * - Single session at a time - rejects additional connections
 * - Chat ID learned from TELEGRAM_CHAT_ID env or first Telegram message
 */

import { WebSocketServer, WebSocket } from 'ws';
import * as TelegramBotModule from 'node-telegram-bot-api';
import { config } from 'dotenv';
import {
  parseAllowedUserIds,
  isUserAuthorized,
  getWhitelistStartupMessage,
} from './telegram-whitelist';
import { isSlashCommand, handleSlashCommand } from './telegram-slash-commands';
import { ThinkingBlockHandler } from './telegram-thinking-handler';
import { summarizeToolResult } from './telegram-content-chunker';

// Handle both ESM and CJS module formats
const TelegramBot =
  (TelegramBotModule as { default?: typeof TelegramBotModule }).default ||
  TelegramBotModule;
type TelegramBotInstance = InstanceType<typeof TelegramBot>;

// Load environment variables
config();

// ============================================================================
// Types
// ============================================================================

export interface StreamChunkData {
  type:
    | 'text'
    | 'thinking'
    | 'tool_call'
    | 'tool_result'
    | 'done'
    | 'error'
    | 'pause_request';
  text?: string;
  thinking?: string;
  name?: string;
  id?: string;
  tool_call_id?: string;
  content?: string;
  is_error?: boolean;
  error?: string;
  // BRIDGE-014: Pause request fields
  pause_kind?: 'triple';
  pause_message?: string;
  pause_tool_name?: string;
  pause_details?: string;
}

export interface StreamChunkMessage {
  type: 'chunk';
  session_id: string;
  data: StreamChunkData;
}

export interface ConnectedMessage {
  type: 'connected';
  session_id: string;
  data: Record<string, never>;
}

export type OutboundMessage = StreamChunkMessage | ConnectedMessage;

export interface InboundMessage {
  type: 'input';
  session_id: string;
  message: string;
  images?: Array<{ data: string; media_type: string }>;
}

export interface EndpointState {
  wss: WebSocketServer | null;
  bot: TelegramBotInstance | null;
  currentSession: {
    ws: WebSocket | null;
    sessionId: string | null;
  };
  chatId: string | null;
  toolNameMap: Map<string, string>;
  isRunning: boolean;
  // Buffering state
  messageBuffer: string[];
  bufferCharCount: number;
  bufferTimer: ReturnType<typeof setTimeout> | null;
  lastSendTime: number;
  lastChunkTime: number;
  // Thinking block handler (manages <think>...</think> tags)
  thinkingHandler: ThinkingBlockHandler;
  // User whitelist
  allowedUserIds: Set<number> | null;
  // Agent state for /status command
  agentState: 'idle' | 'thinking' | 'executing';
  // BRIDGE-014: Pause state management
  isPaused: boolean;
  pauseInfo?: {
    kind: 'triple';
    message: string;
    toolName?: string;
    details?: string;
  };
}

// ============================================================================
// Buffering Configuration
// ============================================================================

const BUFFER_IDLE_FLUSH_MS = 800; // Flush after 800ms of no new chunks (idle)
const MIN_SEND_INTERVAL_MS = 300; // Minimum time between sends
const MAX_BUFFER_SIZE = 50; // Force flush if buffer exceeds this many chunks
const MAX_BUFFER_CHARS = 3500; // Force flush if buffer approaches Telegram limit

// ============================================================================
// Global State
// ============================================================================

const state: EndpointState = {
  wss: null,
  bot: null,
  currentSession: {
    ws: null,
    sessionId: null,
  },
  chatId: null,
  toolNameMap: new Map(),
  isRunning: false,
  messageBuffer: [],
  bufferCharCount: 0,
  bufferTimer: null,
  lastSendTime: 0,
  lastChunkTime: 0,
  thinkingHandler: new ThinkingBlockHandler(),
  allowedUserIds: null,
  agentState: 'idle',
  // BRIDGE-014: Pause state
  isPaused: false,
  pauseInfo: undefined,
};

// ============================================================================
// MarkdownV2 Escaping
// ============================================================================

/**
 * Get media type from file path extension.
 * Defaults to image/jpeg for unknown or missing extensions (Telegram photos are typically JPEG).
 */
export function getMediaTypeFromPath(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'png':
      return 'image/png';
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg';
    case 'gif':
      return 'image/gif';
    case 'webp':
      return 'image/webp';
    default:
      return 'image/jpeg'; // Default for Telegram photos
  }
}

/**
 * Download a photo from Telegram and convert to base64.
 * Returns null if download fails.
 */
export async function downloadPhotoAsBase64(
  bot: TelegramBotInstance,
  fileId: string
): Promise<{ data: string; media_type: string } | null> {
  try {
    const fileLink = await bot.getFileLink(fileId);
    const response = await globalThis.fetch(fileLink);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const buffer = await response.arrayBuffer();
    const base64 = Buffer.from(buffer).toString('base64');
    const mediaType = getMediaTypeFromPath(fileLink);
    return { data: base64, media_type: mediaType };
  } catch (error) {
    console.error('[telegram-endpoint] Photo download failed:', error);
    return null;
  }
}

/**
 * Escape special characters for Telegram MarkdownV2 format.
 * Characters that need escaping: _ * [ ] ( ) ~ ` > # + - = | { } . !
 * Does NOT escape content inside code blocks.
 */
export function escapeMarkdownV2(text: string): string {
  const specialChars = /([_*[\]()~`>#+\-=|{}.!<])/g;

  // Split by code blocks to avoid escaping inside them
  const parts = text.split(/(```[\s\S]*?```|`[^`]+`)/);

  return parts
    .map((part, index) => {
      // Odd indices are code blocks (matched by the regex group)
      if (index % 2 === 1) {
        return part; // Don't escape inside code blocks
      }
      // Note: '\\$1' in JS source becomes '\$1' at runtime (backslash + captured group)
      return part.replace(specialChars, '\\$1');
    })
    .join('');
}

// ============================================================================
// Message Truncation
// ============================================================================

const TELEGRAM_MAX_LENGTH = 4096;
const PRESERVE_CHARS = 1500;

/**
 * Smart truncation that preserves beginning and end of message.
 * Properly handles code block boundaries.
 */
export function truncateMessage(
  text: string,
  maxLength: number = TELEGRAM_MAX_LENGTH
): string {
  if (text.length <= maxLength) {
    return text;
  }

  // Calculate how much we need to cut
  const omittedChars = text.length - PRESERVE_CHARS * 2;
  const indicator = `\n\n[...${omittedChars} chars omitted...]\n\n`;
  const targetLength = maxLength - indicator.length;
  const halfTarget = Math.floor(targetLength / 2);

  let beginning = text.slice(0, halfTarget);
  let ending = text.slice(-halfTarget);

  // Handle code block boundaries in the beginning part
  const beginningOpenBlocks = countOpenCodeBlocks(beginning);
  if (beginningOpenBlocks > 0) {
    // Close any open code blocks
    beginning += '\n```';
  }

  // Handle code block boundaries in the ending part
  const endingHasOpenBlock = hasUnclosedCodeBlock(ending);
  if (endingHasOpenBlock || beginningOpenBlocks > 0) {
    // Re-open code block if the ending contains code block content
    const firstCodeBlockInEnding = ending.indexOf('```');
    if (firstCodeBlockInEnding === -1 || endingHasOpenBlock) {
      ending = '```\n' + ending;
    }
  }

  return beginning + indicator + ending;
}

/**
 * Count unclosed code blocks (``` markers) in text.
 */
function countOpenCodeBlocks(text: string): number {
  const matches = text.match(/```/g);
  if (!matches) {
    return 0;
  }
  // Odd number means there's an unclosed block
  return matches.length % 2;
}

/**
 * Check if text starts inside an unclosed code block.
 * (i.e., the text would need an opening ``` prepended)
 */
function hasUnclosedCodeBlock(text: string): boolean {
  // Check if first ``` is a closing marker (no language specifier before it)
  const firstMarker = text.indexOf('```');
  if (firstMarker === -1) {
    return false;
  }
  // If there's text before the first ```, check if it looks like code block content
  const beforeMarker = text.slice(0, firstMarker);
  // If the text before contains newlines but no ```, we're inside a code block
  return beforeMarker.includes('\n') && !beforeMarker.includes('```');
}

// ============================================================================
// Message Formatting
// ============================================================================

/**
 * Format a StreamChunk for Telegram display.
 */
export function formatForTelegram(
  chunk: StreamChunkData,
  toolNameMap?: Map<string, string>
): string {
  const map = toolNameMap || state.toolNameMap;

  switch (chunk.type) {
    case 'text':
      return escapeMarkdownV2(chunk.text || '');

    case 'thinking':
      return `💭 ${escapeMarkdownV2(chunk.thinking || '')}`;

    case 'tool_call':
      // Store tool name for later correlation
      if (chunk.id && chunk.name) {
        map.set(chunk.id, chunk.name);
      }
      return `🔧 Running: ${escapeMarkdownV2(chunk.name || 'unknown')}`;

    case 'tool_result': {
      // Look up tool name from stored mapping
      const toolName = chunk.tool_call_id
        ? map.get(chunk.tool_call_id) || 'unknown'
        : 'unknown';
      const content = chunk.content || '';
      const prefix = chunk.is_error ? '❌ ' : '';
      const lineCount = (content.match(/\n/g) || []).length + 1;

      // Summarize large outputs instead of sending verbatim
      if (content.length > 500 || lineCount > 20) {
        const summary = summarizeToolResult(toolName, content);
        return `${prefix}${escapeMarkdownV2(summary)}`;
      }

      return `${prefix}\\[${escapeMarkdownV2(toolName)}\\] ${escapeMarkdownV2(content)}`;
    }

    case 'done':
      return '✓';

    case 'error':
      return `❌ Error: ${escapeMarkdownV2(chunk.error || 'Unknown error')}`;

    default:
      return '';
  }
}

// ============================================================================
// Stream Chunk Handling with Buffering
// ============================================================================

/**
 * Flush the message buffer to Telegram.
 * Combines all buffered chunks into a single message.
 *
 * NOTE: Does NOT close thinking blocks - thinking content streams naturally
 * across multiple flushes. Thinking blocks are only closed when transitioning
 * to non-thinking content (see handleStreamChunk).
 */
async function flushBuffer(): Promise<void> {
  // Clear the timer
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
    state.bufferTimer = null;
  }

  // Nothing to flush
  if (state.messageBuffer.length === 0) {
    return;
  }

  if (!state.bot || !state.chatId) {
    state.messageBuffer = [];
    state.bufferCharCount = 0;
    return;
  }

  // Combine all buffered messages
  const combined = state.messageBuffer.join('');
  state.messageBuffer = [];
  state.bufferCharCount = 0;

  if (!combined.trim()) {
    return;
  }

  // Truncate if necessary
  const text = truncateMessage(combined);

  // Send to Telegram
  try {
    state.lastSendTime = Date.now();
    await state.bot.sendMessage(state.chatId, text, {
      parse_mode: 'MarkdownV2',
    });
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error('[telegram-endpoint] Telegram API error:', errorMessage);
    // Drop the message and continue
  }
}

/**
 * Schedule a buffer flush based on idle time.
 * Resets the timer each time new content arrives, so we only flush
 * after a period of no new chunks (idle flush).
 */
function scheduleIdleFlush(): void {
  // Clear any existing timer - we reset on each new chunk
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
  }

  // Calculate delay - respect minimum send interval
  const timeSinceLastSend = Date.now() - state.lastSendTime;
  const delay = Math.max(
    BUFFER_IDLE_FLUSH_MS,
    MIN_SEND_INTERVAL_MS - timeSinceLastSend
  );

  state.bufferTimer = setTimeout(() => {
    void flushBuffer();
  }, delay);
}

/**
 * Handle an outbound message (codelet → Telegram).
 * Buffers messages and flushes on idle (no new chunks for a period)
 * or when buffer size limits are reached.
 *
 * Thinking blocks are wrapped in <think>...</think> tags using ThinkingBlockHandler.
 */
export async function handleStreamChunk(
  msg: StreamChunkMessage
): Promise<void> {
  if (!state.bot) {
    console.error('[telegram-endpoint] Bot not initialized');
    return;
  }

  if (!state.chatId) {
    console.warn(
      '[telegram-endpoint] No chat ID linked - dropping chunk. Waiting for first Telegram message.'
    );
    return;
  }

  // Update agent state based on chunk type
  if (msg.data.type === 'thinking') {
    state.agentState = 'thinking';
  } else if (msg.data.type === 'tool_call') {
    state.agentState = 'executing';
  } else if (msg.data.type === 'done' || msg.data.type === 'error') {
    state.agentState = 'idle';
    // BRIDGE-014: Clear pause state on done/error
    state.isPaused = false;
    state.pauseInfo = undefined;
  }

  // BRIDGE-014: Handle pause_request chunks
  if (msg.data.type === 'pause_request') {
    state.isPaused = true;
    state.pauseInfo = {
      kind: msg.data.pause_kind ?? 'triple',
      message: msg.data.pause_message ?? 'Waiting for access decision',
      toolName: msg.data.pause_tool_name,
      details: msg.data.pause_details,
    };

    // Send pause notification to Telegram
    const toolName = msg.data.pause_tool_name ?? 'Tool';
    const pauseMessage = msg.data.pause_message ?? 'Sensitive file access';
    const text = `⏸ ${escapeMarkdownV2(toolName)}: ${escapeMarkdownV2(pauseMessage)}\n\nRespond with /allowonce, /allowsession, or /deny`;

    if (state.bot && state.chatId) {
      try {
        await state.bot.sendMessage(state.chatId, text, {
          parse_mode: 'MarkdownV2',
        });
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        console.error(
          '[telegram-endpoint] Failed to send pause notification:',
          errorMessage
        );
      }
    }
    return;
  }

  // Track when we last received a chunk
  state.lastChunkTime = Date.now();

  // Handle thinking chunks with ThinkingBlockHandler
  if (msg.data.type === 'thinking') {
    const escapedContent = escapeMarkdownV2(msg.data.thinking || '');
    const formatted = state.thinkingHandler.processThinking(escapedContent);

    state.messageBuffer.push(formatted);
    state.bufferCharCount += formatted.length;

    // Force flush if buffer exceeds limits
    if (
      state.bufferCharCount >= MAX_BUFFER_CHARS ||
      state.messageBuffer.length >= MAX_BUFFER_SIZE
    ) {
      await flushBuffer();
    } else {
      scheduleIdleFlush();
    }
    return;
  }

  // Format the message (for non-thinking chunk types)
  const text = formatForTelegram(msg.data);

  // IMPORTANT: Only close thinking block if there's actual content to transition to.
  // Empty chunks (e.g., empty text) should NOT close thinking blocks.
  if (!text) {
    return;
  }

  // Close thinking block when transitioning to non-thinking content WITH actual content
  const closeTag = state.thinkingHandler.closeIfOpen();
  if (closeTag) {
    state.messageBuffer.push(closeTag);
    state.bufferCharCount += closeTag.length;
  }

  // Handle special chunk types that should flush immediately
  if (msg.data.type === 'done' || msg.data.type === 'error') {
    // Add to buffer and flush immediately
    state.messageBuffer.push(text);
    state.bufferCharCount += text.length;
    await flushBuffer();
    return;
  }

  // Handle tool_call and tool_result - these are important markers
  // Add them to buffer but potentially flush if buffer is getting large
  if (msg.data.type === 'tool_call' || msg.data.type === 'tool_result') {
    // Add newline before tool markers for readability
    if (state.messageBuffer.length > 0) {
      state.messageBuffer.push('\n');
      state.bufferCharCount += 1;
    }
    state.messageBuffer.push(text);
    state.messageBuffer.push('\n');
    state.bufferCharCount += text.length + 1;

    // Force flush if buffer is approaching Telegram's limit
    if (
      state.bufferCharCount >= MAX_BUFFER_CHARS ||
      state.messageBuffer.length >= MAX_BUFFER_SIZE
    ) {
      await flushBuffer();
    } else {
      scheduleIdleFlush();
    }
    return;
  }

  // Regular text chunks - buffer them
  state.messageBuffer.push(text);
  state.bufferCharCount += text.length;

  // Force flush if buffer exceeds limits
  if (
    state.bufferCharCount >= MAX_BUFFER_CHARS ||
    state.messageBuffer.length >= MAX_BUFFER_SIZE
  ) {
    await flushBuffer();
  } else {
    // Schedule idle flush - timer resets on each new chunk
    scheduleIdleFlush();
  }
}

// ============================================================================
// Telegram Message Handling
// ============================================================================

/**
 * Send a control message to the WebSocket session.
 * Used by slash commands (/stop, /clear, /allowonce, /allowsession, /deny) to communicate with the agent.
 */
function sendControlMessage(
  ws: WebSocket,
  sessionId: string,
  action: string,
  response?: string
): void {
  const message: Record<string, string> = {
    type: 'control',
    action,
    session_id: sessionId,
  };
  if (response !== undefined) {
    message.response = response;
  }
  ws.send(JSON.stringify(message));
}

/**
 * Handle an inbound message from Telegram.
 * Updates active chat ID and returns message to send to codelet.
 */
export function handleTelegramMessage(
  chatId: string,
  text: string,
  images?: Array<{ data: string; media_type: string }>
): InboundMessage {
  // Update active chat ID (allows device switching)
  state.chatId = chatId;

  // Create message for codelet
  const message: InboundMessage = {
    type: 'input',
    session_id: state.currentSession.sessionId || '',
    message: text,
  };

  // Only include images if provided and non-empty
  if (images && images.length > 0) {
    message.images = images;
  }

  return message;
}

// ============================================================================
// WebSocket Server
// ============================================================================

function setupWebSocketServer(port: number, host: string): WebSocketServer {
  const wss = new WebSocketServer({ port, host });

  wss.on('connection', (ws: WebSocket) => {
    // Check if a session is already connected
    if (state.currentSession.ws !== null) {
      console.log(
        '[telegram-endpoint] Rejecting connection - session already active'
      );
      ws.close(4000, 'Session already connected');
      return;
    }

    console.log('[telegram-endpoint] Codelet session connected');
    state.currentSession.ws = ws;

    ws.on('message', (data: Buffer | string) => {
      try {
        const message = JSON.parse(data.toString()) as OutboundMessage;
        if (message.type === 'connected') {
          // Learn session ID from connection handshake
          state.currentSession.sessionId = message.session_id;
          console.log(
            `[telegram-endpoint] Session connected: ${message.session_id}`
          );
        } else if (message.type === 'chunk') {
          void handleStreamChunk(message);
        }
      } catch (error) {
        console.error('[telegram-endpoint] Failed to parse message:', error);
      }
    });

    ws.on('close', () => {
      console.log('[telegram-endpoint] Codelet session disconnected');
      // Close any open thinking block before flushing
      const closeTag = state.thinkingHandler.closeIfOpen();
      if (closeTag) {
        state.messageBuffer.push(closeTag);
        state.bufferCharCount += closeTag.length;
      }
      // Flush any remaining buffered messages before clearing state
      void flushBuffer();
      state.currentSession.ws = null;
      state.currentSession.sessionId = null;
      state.toolNameMap.clear();
      state.thinkingHandler.reset();
    });

    ws.on('error', error => {
      console.error('[telegram-endpoint] WebSocket error:', error);
    });
  });

  wss.on('error', error => {
    console.error('[telegram-endpoint] WebSocket server error:', error);
  });

  return wss;
}

// ============================================================================
// Telegram Bot Setup
// ============================================================================

function setupTelegramBot(token: string): TelegramBotInstance {
  // Force IPv4 to avoid Node.js 22 Happy Eyeballs timeout issues with dual-stack DNS
  // The 'family' option forces IPv4-only connections
  // Cast needed because @types/node-telegram-bot-api expects full request.Options
  // but the library actually accepts partial options at runtime
  const requestOptions = {
    family: 4,
  } as TelegramBotModule.ConstructorOptions['request'];
  const bot = new TelegramBot(token, {
    polling: true,
    request: requestOptions,
  });

  bot.on('message', async msg => {
    const chatId = msg.chat.id.toString();

    // User ID validation (whitelist check) using extracted pure function
    const authResult = isUserAuthorized(msg.from?.id, state.allowedUserIds);
    if (!authResult.authorized) {
      console.log(`[telegram-endpoint] Dropping message: ${authResult.reason}`);
      return;
    }

    // Check for photo message
    if (msg.photo && msg.photo.length > 0) {
      // Use caption for photos, not text (msg.text is undefined for photos)
      const text = msg.caption || '';

      console.log(
        `[telegram-endpoint] Received photo from chat ${chatId}${text ? `: "${text.slice(0, 50)}..."` : ''}`
      );

      // Get highest resolution (last element in array)
      const highestRes = msg.photo[msg.photo.length - 1];

      // Download and convert to base64
      let images: Array<{ data: string; media_type: string }> = [];
      const imageData = await downloadPhotoAsBase64(bot, highestRes.file_id);
      if (imageData) {
        images = [imageData];
      } else {
        console.error(
          '[telegram-endpoint] Photo download failed, forwarding caption only'
        );
      }

      // Don't send if no caption and no image
      if (!text && images.length === 0) {
        console.warn(
          '[telegram-endpoint] Photo download failed with no caption, dropping message'
        );
        return;
      }

      // Update active chat ID
      state.chatId = chatId;

      // If we have a connected session, forward the message
      if (
        state.currentSession.ws &&
        state.currentSession.ws.readyState === WebSocket.OPEN
      ) {
        const inputMessage = handleTelegramMessage(chatId, text, images);
        console.log(
          `[telegram-endpoint] Sending photo input to codelet - session_id: "${inputMessage.session_id}", caption: "${text.slice(0, 50)}...", images: ${images.length}`
        );
        state.currentSession.ws.send(JSON.stringify(inputMessage));
      } else {
        console.log(
          '[telegram-endpoint] No active session to forward photo to'
        );
      }
      return;
    }

    // Regular text message
    const text = msg.text || '';

    console.log(
      `[telegram-endpoint] Received message from chat ${chatId}: ${text.slice(0, 50)}...`
    );

    // Update active chat ID
    state.chatId = chatId;

    // Handle slash commands before forwarding to agent
    if (isSlashCommand(text)) {
      console.log(`[telegram-endpoint] Processing slash command: ${text}`);
      const result = await handleSlashCommand(text, state);
      if (result.handled) {
        // Send control message to session if action is required
        if (result.action && state.currentSession.ws) {
          // BRIDGE-014: Handle pause response actions
          if (
            result.action === 'allow_once' ||
            result.action === 'allow_session' ||
            result.action === 'deny'
          ) {
            // Clear pause state
            state.isPaused = false;
            state.pauseInfo = undefined;
            // Send pause_response control message
            sendControlMessage(
              state.currentSession.ws,
              state.currentSession.sessionId || '',
              'pause_response',
              result.action
            );
          } else {
            // Original actions (stop, clear)
            const actionMap: Record<string, string> = {
              stop: 'interrupt',
              clear: 'clear',
            };
            sendControlMessage(
              state.currentSession.ws,
              state.currentSession.sessionId || '',
              actionMap[result.action]
            );
          }
        }
        return; // Don't forward slash commands to agent
      }
    }

    // If we have a connected session, forward the message
    if (
      state.currentSession.ws &&
      state.currentSession.ws.readyState === WebSocket.OPEN
    ) {
      // BRIDGE-011: Set agentState to 'thinking' immediately when forwarding a message
      // This ensures /stop correctly detects the agent is processing before any chunks arrive
      state.agentState = 'thinking';

      const inputMessage = handleTelegramMessage(chatId, text);
      console.log(
        `[telegram-endpoint] Sending input to codelet - session_id: "${inputMessage.session_id}", message: "${inputMessage.message.slice(0, 50)}..."`
      );
      state.currentSession.ws.send(JSON.stringify(inputMessage));
    } else {
      console.log(
        '[telegram-endpoint] No active session to forward message to'
      );
    }
  });

  bot.on('polling_error', error => {
    console.error('[telegram-endpoint] Telegram polling error:', error);
  });

  return bot;
}

// ============================================================================
// Endpoint Lifecycle
// ============================================================================

/**
 * Start the Telegram bridge endpoint.
 */
export function startEndpoint(): EndpointState {
  // Validate required environment variables
  const token = process.env.TELEGRAM_BOT_TOKEN;
  if (!token) {
    throw new Error('Missing required TELEGRAM_BOT_TOKEN environment variable');
  }

  // Get configuration from environment
  const port = parseInt(process.env.WEBSOCKET_PORT || '8080', 10);
  const host = process.env.WEBSOCKET_HOST || 'localhost';
  const preConfiguredChatId = process.env.TELEGRAM_CHAT_ID;

  // Initialize chat ID from environment if provided
  if (preConfiguredChatId) {
    state.chatId = preConfiguredChatId;
    console.log(
      `[telegram-endpoint] Pre-configured chat ID: ${preConfiguredChatId}`
    );
  } else {
    console.warn(
      '[telegram-endpoint] No TELEGRAM_CHAT_ID configured - waiting for first Telegram message'
    );
  }

  // Parse allowed user IDs from environment using extracted pure function
  const whitelistResult = parseAllowedUserIds(
    process.env.TELEGRAM_ALLOWED_USER_IDS
  );
  state.allowedUserIds = whitelistResult.allowedUserIds;

  const whitelistMessage = getWhitelistStartupMessage(whitelistResult);
  if (
    whitelistResult.allowedUserIds === null &&
    whitelistResult.invalidIdCount > 0
  ) {
    console.warn(`[telegram-endpoint] ${whitelistMessage}`);
  } else {
    console.log(`[telegram-endpoint] ${whitelistMessage}`);
  }

  // Start WebSocket server
  state.wss = setupWebSocketServer(port, host);
  console.log(
    `[telegram-endpoint] WebSocket server listening on ${host}:${port}`
  );

  // Start Telegram bot
  state.bot = setupTelegramBot(token);
  console.log('[telegram-endpoint] Telegram bot connected with polling mode');

  state.isRunning = true;
  return state;
}

/**
 * Stop the endpoint gracefully.
 */
export async function stopEndpoint(): Promise<void> {
  // Flush any remaining buffered messages
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
    state.bufferTimer = null;
  }
  if (state.messageBuffer.length > 0 && state.bot && state.chatId) {
    await flushBuffer();
  }

  if (state.bot) {
    await state.bot.stopPolling();
    state.bot = null;
  }

  if (state.wss) {
    state.wss.close();
    state.wss = null;
  }

  state.currentSession.ws = null;
  state.currentSession.sessionId = null;
  state.chatId = null;
  state.toolNameMap.clear();
  state.messageBuffer = [];
  state.bufferCharCount = 0;
  state.lastSendTime = 0;
  state.lastChunkTime = 0;
  state.thinkingHandler.reset();
  state.agentState = 'idle';
  // BRIDGE-014: Reset pause state
  state.isPaused = false;
  state.pauseInfo = undefined;
  state.isRunning = false;
}

/**
 * Reset state (for testing).
 */
export function resetState(): void {
  if (state.bufferTimer) {
    clearTimeout(state.bufferTimer);
  }
  state.wss = null;
  state.bot = null;
  state.currentSession = { ws: null, sessionId: null };
  state.chatId = null;
  state.toolNameMap.clear();
  state.messageBuffer = [];
  state.bufferCharCount = 0;
  state.bufferTimer = null;
  state.lastSendTime = 0;
  state.lastChunkTime = 0;
  state.thinkingHandler.reset();
  state.allowedUserIds = null;
  state.agentState = 'idle';
  // BRIDGE-014: Reset pause state
  state.isPaused = false;
  state.pauseInfo = undefined;
  state.isRunning = false;
}

/**
 * Get current state (for testing).
 */
export function getState(): EndpointState {
  return state;
}

// ============================================================================
// CLI Entry Point
// ============================================================================

// Run when executed directly (use environment variable or check process.argv)
const runAsMain = process.argv[1]?.includes('telegram-endpoint');

if (runAsMain) {
  try {
    startEndpoint();
    console.log('[telegram-endpoint] Endpoint started successfully');

    // Handle graceful shutdown
    process.on('SIGINT', async () => {
      console.log('\n[telegram-endpoint] Shutting down...');
      await stopEndpoint();
      process.exit(0);
    });

    process.on('SIGTERM', async () => {
      console.log('[telegram-endpoint] Received SIGTERM, shutting down...');
      await stopEndpoint();
      process.exit(0);
    });
  } catch (error) {
    console.error('[telegram-endpoint] Failed to start:', error);
    process.exit(1);
  }
}
