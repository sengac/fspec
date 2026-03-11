/**
 * Output abstraction for fspec commands.
 *
 * Commands should use `output.log()`, `output.error()`, etc. instead of
 * `console.log()`, `console.error()` directly. This allows the fspec-callback
 * to capture output without hijacking process.stdout/stderr.
 *
 * In CLI mode: writes to console with colors (default)
 * In tool mode: captures to buffer without colors for structured response
 *
 * IMPORTANT: Commands should NOT use chalk with output.log/error/warn.
 * The coloring is handled automatically by the output abstraction.
 */

import chalk from 'chalk';

// Comprehensive ANSI escape sequence regex
// Handles: CSI sequences (colors, cursor, erase, mouse tracking), OSC sequences, other escapes
// prettier-ignore
// eslint-disable-next-line no-control-regex
const ANSI_REGEX = /\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[()#][0-9A-Za-z]|\x1b[^[\]()#]/g;

/**
 * Strip all ANSI escape sequences from a string.
 * Handles SGR colors, CSI cursor/erase/mouse sequences, and OSC sequences.
 */
export function stripAnsi(str: string): string {
  return str.replace(ANSI_REGEX, '');
}

export interface OutputContext {
  log: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
}

// Track if we're in capture mode (fspec tool) vs CLI mode
let isCaptureMode = false;

// Default context: write to console with colors (CLI mode)
const defaultContext: OutputContext = {
  log: (...args: unknown[]) => console.log(...args),
  error: (...args: unknown[]) =>
    console.error(chalk.red(args.map(a => String(a)).join(' '))),
  warn: (...args: unknown[]) =>
    console.warn(chalk.yellow(args.map(a => String(a)).join(' '))),
};

// Current active context
let currentContext: OutputContext = defaultContext;

/**
 * Set the output context for command execution.
 * Call with no arguments to reset to default (console).
 */
export function setOutputContext(ctx?: OutputContext): void {
  currentContext = ctx || defaultContext;
  isCaptureMode = ctx !== undefined && ctx !== defaultContext;
}

/**
 * Get the current output context.
 */
export function getOutputContext(): OutputContext {
  return currentContext;
}

/**
 * Reset output context to default (console).
 */
export function resetOutputContext(): void {
  currentContext = defaultContext;
  isCaptureMode = false;
}

/**
 * Check if we're in capture mode (fspec tool) vs CLI mode.
 */
export function isInCaptureMode(): boolean {
  return isCaptureMode;
}

/**
 * Create a capture context that stores output in arrays.
 * Returns the context and the captured output arrays.
 * In capture mode, no colors are applied - plain text only.
 */
export function createCaptureContext(): {
  context: OutputContext;
  stdout: string[];
  stderr: string[];
} {
  const stdout: string[] = [];
  const stderr: string[] = [];

  const context: OutputContext = {
    log: (...args: unknown[]) => {
      const message = args
        .map(a => (typeof a === 'string' ? a : JSON.stringify(a)))
        .join(' ');
      stdout.push(stripAnsi(message));
    },
    error: (...args: unknown[]) => {
      const message = args
        .map(a => (typeof a === 'string' ? a : JSON.stringify(a)))
        .join(' ');
      stderr.push(stripAnsi(message));
    },
    warn: (...args: unknown[]) => {
      const message = args
        .map(a => (typeof a === 'string' ? a : JSON.stringify(a)))
        .join(' ');
      stderr.push(stripAnsi(message));
    },
  };

  return { context, stdout, stderr };
}

/**
 * Output object for commands to use.
 * Commands should import this and use output.log(), output.error(), etc.
 *
 * IMPORTANT: Do NOT wrap output.log/error/warn calls with chalk.
 * Coloring is handled automatically based on CLI vs tool mode.
 */
export const output = {
  log: (...args: unknown[]): void => currentContext.log(...args),
  error: (...args: unknown[]): void => currentContext.error(...args),
  warn: (...args: unknown[]): void => currentContext.warn(...args),
};

// =============================================================================
// Fspec Context - for passing args from fspec-callback to commands
// =============================================================================
// RES-022: Research tools were using process.argv instead of Commander args
// when invoked via Fspec tool. This context allows fspec-callback to pass
// the positional args to commands that need them.

/** Positional args passed from fspec-callback (null when not in fspec tool mode) */
let fspecPositionalArgs: string[] | null = null;

/**
 * Set the positional args for the current fspec-callback execution.
 * Called by fspec-callback before executing a command.
 */
export function setFspecPositionalArgs(args: string[] | null): void {
  fspecPositionalArgs = args;
}

/**
 * Get the positional args set by fspec-callback, or null if not in fspec tool mode.
 * Commands that need to forward args (like research) should check this first
 * before falling back to process.argv.
 */
export function getFspecPositionalArgs(): string[] | null {
  return fspecPositionalArgs;
}

/**
 * Clear the fspec positional args.
 * Called by fspec-callback after executing a command.
 */
export function clearFspecPositionalArgs(): void {
  fspecPositionalArgs = null;
}
