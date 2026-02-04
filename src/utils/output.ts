/**
 * Output abstraction for fspec commands.
 *
 * Commands should use `output.log()`, `output.error()`, etc. instead of
 * `console.log()`, `console.error()` directly. This allows the fspec-callback
 * to capture output without hijacking process.stdout/stderr.
 *
 * In CLI mode: writes to console (default)
 * In tool mode: captures to buffer for structured response
 */

export interface OutputContext {
  log: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
}

// Default context: write to console (CLI mode)
const defaultContext: OutputContext = {
  log: (...args: unknown[]) => console.log(...args),
  error: (...args: unknown[]) => console.error(...args),
  warn: (...args: unknown[]) => console.warn(...args),
};

// Current active context
let currentContext: OutputContext = defaultContext;

/**
 * Set the output context for command execution.
 * Call with no arguments to reset to default (console).
 */
export function setOutputContext(ctx?: OutputContext): void {
  currentContext = ctx || defaultContext;
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
}

/**
 * Create a capture context that stores output in arrays.
 * Returns the context and the captured output arrays.
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
      stdout.push(message);
    },
    error: (...args: unknown[]) => {
      const message = args
        .map(a => (typeof a === 'string' ? a : JSON.stringify(a)))
        .join(' ');
      stderr.push(message);
    },
    warn: (...args: unknown[]) => {
      const message = args
        .map(a => (typeof a === 'string' ? a : JSON.stringify(a)))
        .join(' ');
      stderr.push(message);
    },
  };

  return { context, stdout, stderr };
}

/**
 * Output object for commands to use.
 * Commands should import this and use output.log(), output.error(), etc.
 */
export const output = {
  log: (...args: unknown[]): void => currentContext.log(...args),
  error: (...args: unknown[]): void => currentContext.error(...args),
  warn: (...args: unknown[]): void => currentContext.warn(...args),
};
