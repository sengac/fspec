/**
 * Slash Command Template
 *
 * Embedded from .claude/commands/fspec.md to avoid filesystem dependencies.
 * This file is large (~1000 lines) but necessary for bundle-safe operation.
 */

import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// Read template at module load time (Vite will bundle this)
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Development: read from .claude/commands/fspec.md
// Production: this gets bundled by Vite
let SLASH_COMMAND_TEMPLATE: string;

try {
  const templatePath = join(__dirname, '..', '..', '.claude', 'commands', 'fspec.md');
  SLASH_COMMAND_TEMPLATE = readFileSync(templatePath, 'utf-8');
} catch {
  // Fallback if file not found (shouldn't happen in dev or production)
  SLASH_COMMAND_TEMPLATE = '# fspec Command\n\nBasic fallback template.';
}

export function getSlashCommandTemplate(): string {
  return SLASH_COMMAND_TEMPLATE;
}
