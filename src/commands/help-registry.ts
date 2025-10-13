/**
 * Registry of commands with custom help configs
 *
 * Uses Vite's import.meta.glob to eagerly load all help configs at build time.
 * This ensures all help files are included in the bundle.
 *
 * To add custom help for a command:
 * 1. Create src/commands/command-name-help.ts with CommandHelpConfig export default
 * 2. That's it! The glob pattern will automatically include it
 */

import type { CommandHelpConfig } from '../utils/help-formatter';

// Eagerly import all help config files
const helpModules = import.meta.glob<{ default: CommandHelpConfig }>(
  './*-help.ts',
  { eager: true }
);

// Build registry from imported modules
export const commandsWithCustomHelp = new Set<string>();
export const helpConfigs = new Map<string, CommandHelpConfig>();

for (const [path, module] of Object.entries(helpModules)) {
  // Extract command name from path: ./create-epic-help.ts -> create-epic
  const match = path.match(/\.\/(.+)-help\.ts$/);
  if (match) {
    const commandName = match[1];
    commandsWithCustomHelp.add(commandName);
    helpConfigs.set(commandName, module.default);
  }
}
