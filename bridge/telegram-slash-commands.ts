/**
 * Telegram Slash Commands for Agent Control
 *
 * BRIDGE-010: Telegram Slash Commands for Agent Control
 * BRIDGE-014: Telegram Pause State Management Commands
 *
 * This module handles slash commands sent from Telegram users to control
 * the agent session. Commands are intercepted before being sent to the agent
 * and handled directly by the bridge.
 *
 * Supported commands:
 * - /help - Show available commands
 * - /status - Show agent session state
 * - /stop - Interrupt current agent operation
 * - /clear - Clear conversation history and reset session
 * - /allowonce (or /allow) - Allow sensitive file access once
 * - /allowsession - Allow sensitive file access for the session
 * - /deny - Deny sensitive file access
 */

import { escapeMarkdownV2 } from './telegram-formatting';

// ============================================================================
// Types
// ============================================================================

export type AgentState = 'idle' | 'thinking' | 'executing';

/**
 * Pause info for sensitive file access prompts (BRIDGE-014)
 */
export interface PauseInfo {
  kind: 'triple';
  message: string;
  toolName?: string;
  details?: string;
}

/**
 * Minimal bot interface - only methods we actually use.
 * This allows proper mocking without type hacks.
 */
export interface MinimalBot {
  sendMessage(
    chatId: string | number,
    text: string,
    options?: { parse_mode?: string }
  ): Promise<unknown>;
}

/**
 * Minimal WebSocket interface - only properties we actually use.
 * This allows proper mocking without type hacks.
 */
export interface MinimalWebSocket {
  readyState: number;
  send(data: string): void;
}

export interface SlashCommandState {
  bot: MinimalBot | null;
  chatId: string | null;
  currentSession: {
    ws: MinimalWebSocket | null;
    sessionId: string | null;
  };
  isRunning: boolean;
  agentState: AgentState;
  /** BRIDGE-014: Whether agent is paused waiting for access decision */
  isPaused?: boolean;
  /** BRIDGE-014: Information about the current pause prompt */
  pauseInfo?: PauseInfo;
}

export interface SlashCommandResult {
  /** Whether the command was handled (true for slash commands, false for regular messages) */
  handled: boolean;
  /** The response sent to the user (if any) */
  response?: string;
  /** Action to perform (for commands that need session interaction) */
  action?: 'stop' | 'clear' | 'allow_once' | 'allow_session' | 'deny';
}

// ============================================================================
// Constants
// ============================================================================

const AVAILABLE_COMMANDS = [
  { command: '/help', description: 'Show this help message' },
  { command: '/status', description: 'Show agent session state' },
  { command: '/stop', description: 'Interrupt current agent operation' },
  {
    command: '/clear',
    description: 'Clear conversation history and reset session',
  },
  {
    command: '/allowonce',
    description: 'Allow sensitive file access once (alias: /allow)',
  },
  {
    command: '/allowsession',
    description: 'Allow sensitive file access for session',
  },
  { command: '/deny', description: 'Deny sensitive file access' },
];

// ============================================================================
// Command Handlers
// ============================================================================

/**
 * Handle /help command - show available commands
 */
function handleHelpCommand(): string {
  const lines = ['*Available commands:*', ''];
  for (const cmd of AVAILABLE_COMMANDS) {
    lines.push(
      `${escapeMarkdownV2(cmd.command)} \\- ${escapeMarkdownV2(cmd.description)}`
    );
  }
  return lines.join('\n');
}

/**
 * Handle /status command - show agent state
 */
function handleStatusCommand(
  agentState: AgentState,
  isPaused?: boolean,
  pauseInfo?: PauseInfo
): string {
  // BRIDGE-014: Check if agent is paused first
  if (isPaused) {
    const message = pauseInfo?.message ?? 'Waiting for access decision';
    return `⏸ Paused: ${escapeMarkdownV2(message)}`;
  }

  switch (agentState) {
    case 'idle':
      return '🟢 Agent is idle';
    case 'thinking':
      return '💭 Agent is thinking\\.\\.\\.';
    case 'executing':
      return '🔧 Agent is executing a tool';
    default:
      return '❓ Unknown agent state';
  }
}

/**
 * Handle /stop command - interrupt agent operation
 */
function handleStopCommand(agentState: AgentState): {
  response: string;
  action?: 'stop';
} {
  if (agentState === 'idle') {
    return { response: '⚠️ Nothing to stop \\- agent is idle' };
  }
  return {
    response: '🛑 Operation stopped',
    action: 'stop',
  };
}

/**
 * Handle /clear command - reset session
 */
function handleClearCommand(): { response: string; action: 'clear' } {
  return {
    response: '🗑️ Session cleared',
    action: 'clear',
  };
}

/**
 * Handle /allowonce command - allow sensitive file access once
 * BRIDGE-014: Responds to PauseKind::Triple prompts
 */
function handleAllowOnceCommand(isPaused?: boolean): {
  response: string;
  action?: 'allow_once';
} {
  if (!isPaused) {
    return { response: '⚠️ No pending pause to respond to' };
  }
  return {
    response: '✅ Access allowed \\(once\\)',
    action: 'allow_once',
  };
}

/**
 * Handle /allowsession command - allow sensitive file access for session
 * BRIDGE-014: Responds to PauseKind::Triple prompts
 */
function handleAllowSessionCommand(isPaused?: boolean): {
  response: string;
  action?: 'allow_session';
} {
  if (!isPaused) {
    return { response: '⚠️ No pending pause to respond to' };
  }
  return {
    response: '✅ Access allowed \\(session\\)',
    action: 'allow_session',
  };
}

/**
 * Handle /deny command - deny sensitive file access
 * BRIDGE-014: Responds to PauseKind::Triple prompts
 */
function handleDenyCommand(isPaused?: boolean): {
  response: string;
  action?: 'deny';
} {
  if (!isPaused) {
    return { response: '⚠️ No pending pause to respond to' };
  }
  return {
    response: '🚫 Access denied',
    action: 'deny',
  };
}

/**
 * Handle unknown command - show error with available commands
 */
function handleUnknownCommand(command: string): string {
  const commandList = AVAILABLE_COMMANDS.map(c =>
    escapeMarkdownV2(c.command)
  ).join(', ');
  return `❌ Unknown command: ${escapeMarkdownV2(command)}\n\nAvailable commands: ${commandList}`;
}

// ============================================================================
// Main Handler
// ============================================================================

/**
 * Check if a message is a slash command
 */
export function isSlashCommand(text: string): boolean {
  return text.startsWith('/');
}

/**
 * Parse a slash command from text
 * Returns the command (lowercase) and any arguments
 */
export function parseSlashCommand(text: string): {
  command: string;
  args: string[];
} {
  const parts = text.trim().split(/\s+/);
  const command = parts[0].toLowerCase();
  const args = parts.slice(1);
  return { command, args };
}

/**
 * Handle a slash command from Telegram
 *
 * @param text The message text from Telegram
 * @param state The current endpoint state
 * @returns Result indicating whether command was handled and any actions to perform
 */
export async function handleSlashCommand(
  text: string,
  state: SlashCommandState
): Promise<SlashCommandResult> {
  // Check if this is a slash command
  if (!isSlashCommand(text)) {
    return { handled: false };
  }

  const { command } = parseSlashCommand(text);
  let response: string;
  let action:
    | 'stop'
    | 'clear'
    | 'allow_once'
    | 'allow_session'
    | 'deny'
    | undefined;

  // Handle each command
  switch (command) {
    case '/help':
      response = handleHelpCommand();
      break;

    case '/status':
      response = handleStatusCommand(
        state.agentState,
        state.isPaused,
        state.pauseInfo
      );
      break;

    case '/stop': {
      const stopResult = handleStopCommand(state.agentState);
      response = stopResult.response;
      action = stopResult.action;
      break;
    }

    case '/clear': {
      const clearResult = handleClearCommand();
      response = clearResult.response;
      action = clearResult.action;
      break;
    }

    // BRIDGE-014: Pause response commands
    case '/allowonce':
    case '/allow': {
      const allowOnceResult = handleAllowOnceCommand(state.isPaused);
      response = allowOnceResult.response;
      action = allowOnceResult.action;
      break;
    }

    case '/allowsession': {
      const allowSessionResult = handleAllowSessionCommand(state.isPaused);
      response = allowSessionResult.response;
      action = allowSessionResult.action;
      break;
    }

    case '/deny': {
      const denyResult = handleDenyCommand(state.isPaused);
      response = denyResult.response;
      action = denyResult.action;
      break;
    }

    default:
      response = handleUnknownCommand(command);
  }

  // Send response to Telegram
  if (state.bot && state.chatId) {
    await state.bot.sendMessage(state.chatId, response, {
      parse_mode: 'MarkdownV2',
    });
  }

  return {
    handled: true,
    response,
    action,
  };
}
