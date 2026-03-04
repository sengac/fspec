/**
 * Codex Allowlist Service
 *
 * PROV-034: Loads and applies the Codex model allowlist for filtering
 * models.dev catalog entries to only Codex-supported models.
 *
 * The allowlist is loaded from:
 * 1. User override: ~/.fspec/codex-models.json (takes precedence)
 * 2. Bundled default: src/tui/data/codex-models.json (fallback)
 *
 * Filtering uses slug prefix matching (like the Codex CLI):
 * e.g., allowlist entry 'gpt-5.2-codex' matches model 'gpt-5.2-codex-2026-03-01'
 */

import { readFile } from 'fs/promises';
import { join } from 'path';
import { getFspecUserDir } from '../../utils/config';
import { logger } from '../../utils/logger';
import type { NapiModelInfo } from '@sengac/codelet-napi';

// Import bundled default as a static JSON module
import bundledAllowlist from '../data/codex-models.json';

// =============================================================================
// TYPES
// =============================================================================

export interface CodexModelEntry {
  slug: string;
  visibility: string;
  priority: number;
}

interface CodexAllowlistConfig {
  version: number;
  description?: string;
  models: CodexModelEntry[];
}

// =============================================================================
// ALLOWLIST LOADING
// =============================================================================

/**
 * Load the Codex model allowlist.
 *
 * Checks ~/.fspec/codex-models.json first (user override),
 * falls back to bundled default.
 *
 * @returns Array of CodexModelEntry objects with slug, visibility, and priority
 */
export async function loadCodexAllowlist(): Promise<CodexModelEntry[]> {
  // Try user override first
  try {
    const userConfigPath = join(getFspecUserDir(), 'codex-models.json');
    const content = await readFile(userConfigPath, 'utf-8');
    const config = JSON.parse(content) as CodexAllowlistConfig;

    if (config.models && Array.isArray(config.models)) {
      logger.debug(
        `PROV-034: Loaded ${config.models.length} Codex model entries from user config`
      );
      return config.models;
    }
  } catch {
    // User config doesn't exist or is invalid — fall through to bundled
  }

  // Fall back to bundled default
  const config = bundledAllowlist as CodexAllowlistConfig;
  logger.debug(
    `PROV-034: Loaded ${config.models.length} Codex model entries from bundled default`
  );
  return config.models;
}

// =============================================================================
// FILTERING
// =============================================================================

/**
 * Check if a model ID matches any entry in the Codex allowlist.
 *
 * Uses slug prefix matching with date-suffix validation:
 * - Exact match: 'gpt-5.2-codex' matches 'gpt-5.2-codex'
 * - Date suffix: 'gpt-5.2-codex' matches 'gpt-5.2-codex-2026-03-01'
 *
 * Does NOT match different model families:
 * - 'gpt-5' does NOT match 'gpt-5-mini' (different model, not a dated variant)
 *
 * Date suffixes recognized: YYYY-MM-DD and YYYYMMDD
 *
 * @param modelId - The model ID from models.dev
 * @param allowlist - Array of CodexModelEntry objects
 * @returns true if the model matches a visible entry in the allowlist
 */
export function matchesCodexAllowlist(
  modelId: string,
  allowlist: CodexModelEntry[]
): boolean {
  return allowlist.some(entry => {
    // Skip hidden models — only 'list' visibility passes through
    if (entry.visibility !== 'list') {
      return false;
    }
    // Exact match
    if (modelId === entry.slug) {
      return true;
    }
    // Prefix match: model must start with slug + '-'
    if (!modelId.startsWith(entry.slug + '-')) {
      return false;
    }
    // Validate suffix is a date pattern (not a different model variant)
    // e.g., '2026-03-01' or '20260301' — not 'mini', 'nano', 'pro'
    const suffix = modelId.slice(entry.slug.length + 1);
    return /^\d{4}-\d{2}-\d{2}$/.test(suffix) || /^\d{8}$/.test(suffix);
  });
}

/**
 * Filter models against the Codex allowlist.
 *
 * Only models whose ID matches (exact or prefix) a visible entry in the
 * allowlist will pass through. Models are sorted by priority (lower = higher priority).
 *
 * @param models - Array of models from models.dev
 * @param allowlist - Array of CodexModelEntry objects with slug, visibility, priority
 * @returns Filtered and priority-sorted array containing only Codex-supported visible models
 */
export function filterByCodexAllowlist(
  models: NapiModelInfo[],
  allowlist: CodexModelEntry[]
): NapiModelInfo[] {
  const filtered = models.filter(m => matchesCodexAllowlist(m.id, allowlist));

  // Sort by Codex catalog priority (lower number = higher priority)
  filtered.sort((a, b) => {
    const priorityA = getModelPriority(a.id, allowlist);
    const priorityB = getModelPriority(b.id, allowlist);
    return priorityA - priorityB;
  });

  logger.debug(
    `PROV-034: Filtered ${models.length} models to ${filtered.length} Codex-supported visible models`
  );
  return filtered;
}

/**
 * Get the priority of a model from the allowlist.
 * Uses the same slug matching logic as matchesCodexAllowlist.
 *
 * @returns priority number, or Infinity if not found
 */
function getModelPriority(
  modelId: string,
  allowlist: CodexModelEntry[]
): number {
  for (const entry of allowlist) {
    if (entry.visibility !== 'list') {
      continue;
    }
    if (modelId === entry.slug) {
      return entry.priority;
    }
    if (modelId.startsWith(entry.slug + '-')) {
      const suffix = modelId.slice(entry.slug.length + 1);
      if (/^\d{4}-\d{2}-\d{2}$/.test(suffix) || /^\d{8}$/.test(suffix)) {
        return entry.priority;
      }
    }
  }
  return Infinity;
}
