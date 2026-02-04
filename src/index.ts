#!/usr/bin/env node

// PERF-001: Clear React 19's performance measure buffer periodically
// React 19's reconciler uses performance.measure() for profiling, which accumulates
// entries over time. After 1,000,000 entries, Node.js emits a warning about potential
// memory leaks. This interval clears the buffer every 30 seconds to prevent the warning.
// NOTE: unref() ensures this interval doesn't keep the process alive - critical for
// CLI commands that should exit after completion (e.g., when run by AI agent tools).
import { performance } from 'perf_hooks';

setInterval(() => {
  performance.clearMeasures();
}, 30000).unref();

// LOG-003: Capture all console methods and redirect to winston logger
// This MUST run before any other imports that might use console to ensure all output is captured
import { initializeConsoleCapture } from './utils/console-capture';
initializeConsoleCapture();

import chalk from 'chalk';
import { fileURLToPath } from 'url';
import { realpathSync } from 'fs';
import { readFileSync } from 'fs';
import { dirname, join } from 'path';
import { render } from 'ink';
import React from 'react';
import { INK_RENDER_OPTIONS } from './tui/config/inkConfig';

// Shared program setup with all commands registered
import { createProgram } from './cli/program';

// Read version from package.json
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const packageJson = JSON.parse(
  readFileSync(join(__dirname, '..', 'package.json'), 'utf-8')
);
const version = packageJson.version;

// Help functions
import { handleHelpCommand } from './help';
import { handleCustomHelp } from './utils/help-interceptor';

// TUI components
import { BoardView } from './tui/components/BoardView';
import { InputProvider } from './tui/components/InputProvider';

// Sync version utility
import { syncVersion } from './commands/sync-version';

// Create program with all commands registered
const program = createProgram();

// Only set version if available
if (version) {
  program.version(version);
}

// Add custom help command
program
  .command('help')
  .description('Display help for command groups')
  .argument('[group]', 'Help topic: spec, tags, foundation, query, project')
  .action((group?: string) => handleHelpCommand(group, version));

async function main(): Promise<void> {
  // Handle --sync-version BEFORE any other processing
  // This must run first to check version and update files if needed
  const syncVersionIndex = process.argv.findIndex(arg =>
    arg.startsWith('--sync-version')
  );
  if (syncVersionIndex !== -1) {
    const versionArg = process.argv[syncVersionIndex];
    const embeddedVersion =
      versionArg.split('=')[1] || process.argv[syncVersionIndex + 1];

    if (embeddedVersion) {
      const exitCode = await syncVersion({ embeddedVersion });
      process.exit(exitCode);
    }
  }

  // Launch interactive TUI when no arguments provided
  // process.argv = ['node', '/path/to/index.js'] when no args
  if (process.argv.length === 2) {
    // Check if stdin supports raw mode (required for Ink)
    // Skip TUI in CI environments or when stdin is not a TTY
    if (!process.stdin.isTTY || process.env.CI === 'true') {
      console.error(
        chalk.yellow('Interactive TUI requires a TTY environment.')
      );
      console.error(
        chalk.yellow('Run with a command or use --help for available commands.')
      );
      process.exit(1);
    }

    // LOG-004: Wire up Rust tracing logs to TypeScript logger
    // Only initialize for TUI mode - CLI commands don't need this and it would
    // prevent the process from exiting due to ThreadsafeFunction references
    const { initializeRustLogCapture } = await import(
      './utils/rust-log-capture'
    );
    initializeRustLogCapture();

    const { waitUntilExit } = render(
      React.createElement(
        InputProvider,
        null,
        React.createElement(BoardView, {
          onExit: () => {
            process.exit(0);
          },
        })
      ),
      {
        // Enable mouse events (trackpad, scroll wheel, clicks)
        stdin: process.stdin,
        stdout: process.stdout,
        // Use shared Ink config to ensure animation timing stays in sync
        ...INK_RENDER_OPTIONS,
      }
    );
    await waitUntilExit();
    return;
  }

  // Handle custom help before Commander.js processes arguments
  const customHelpShown = await handleCustomHelp();

  if (customHelpShown) {
    // Help was displayed and process.exit(0) was called
    return;
  }

  // Normal Commander.js execution
  program.parse();
}

// Run main function when executed directly (not when imported for testing)
// This works for both direct execution (./dist/index.js) and npm link (/usr/local/bin/fspec)
// by checking if the resolved script path matches this file
const isMainModule = (() => {
  try {
    // Resolve symlinks to get the actual file path
    const realArgv1 = realpathSync(process.argv[1]);
    return realArgv1 === __filename;
  } catch {
    // If we can't resolve, fall back to string comparison
    return process.argv[1]?.includes('index.js');
  }
})();

if (isMainModule) {
  main().catch(error => {
    console.error(chalk.red('Fatal error:'), error.message);
    process.exit(1);
  });
}
