/**
 * Slash Command Registry
 *
 * Defines all available slash commands with their names, descriptions,
 * and metadata. Used by SlashCommandPalette for autocomplete.
 *
 * Work Unit: TUI-050
 */

export interface SlashCommand {
  /** Command name (without leading /) */
  name: string;
  /** Human-readable description */
  description: string;
  /** Optional command syntax with arguments */
  syntax?: string;
  /** Optional aliases for quick access */
  aliases?: string[];
  /** Whether command requires an active session (default: true) */
  requiresSession?: boolean;
}

/**
 * All available slash commands in the TUI
 */
export const SLASH_COMMANDS: SlashCommand[] = [
  // Session management
  {
    name: 'model',
    description: 'Select AI model',
    requiresSession: false,
  },
  {
    name: 'provider',
    description: 'Configure API providers',
    requiresSession: false,
  },
  { name: 'debug', description: 'Toggle debug capture mode' },
  { name: 'clear', description: 'Clear conversation history' },
  { name: 'compact', description: 'Compact context window' },

  // Thinking level (TUI-054)
  { name: 'thinking', description: 'Set base thinking level' },

  // Session operations
  { name: 'resume', description: 'Resume a previous session' },
  { name: 'detach', description: 'Detach session from work unit' },

  // History
  { name: 'search', description: 'Search command history' },

  // Supervisors
  { name: 'supervisor', description: 'Manage supervisor sessions' },

  // Blocklist (BLOCK-004)
  { name: 'blocklist', description: 'Manage blocklist rules' },

  // GIT-036: Merge worktree and close session
  {
    name: 'merge-worktree',
    description: 'Merge worktree changes and close session',
  },
];

/**
 * Filter commands using three-tier matching:
 * 1. Exact prefix matches (highest priority)
 * 2. Substring matches in name
 * 3. Substring matches in description (lowest priority)
 *
 * @param commands - Array of commands to filter
 * @param filter - Filter string (text after "/")
 * @returns Filtered and sorted array of matching commands
 */
export function filterCommands(
  commands: SlashCommand[],
  filter: string
): SlashCommand[] {
  if (!filter) {
    return commands;
  }

  const lower = filter.toLowerCase();

  // 1. Exact prefix matches first
  const prefixMatches = commands.filter(c =>
    c.name.toLowerCase().startsWith(lower)
  );

  // 2. Substring matches in name (excluding prefix matches)
  const substringMatches = commands.filter(
    c =>
      !c.name.toLowerCase().startsWith(lower) &&
      c.name.toLowerCase().includes(lower)
  );

  // 3. Description matches (excluding name matches)
  const descMatches = commands.filter(
    c =>
      !c.name.toLowerCase().includes(lower) &&
      c.description.toLowerCase().includes(lower)
  );

  return [...prefixMatches, ...substringMatches, ...descMatches];
}
