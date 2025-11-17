/**
 * Prefill Detection Utility
 *
 * Detects placeholder/prefill text in feature files and provides
 * system-reminders with CLI commands to fix them.
 */

export interface PrefillMatch {
  pattern: string;
  line?: number;
  context?: string;
  suggestion: string;
}

export interface PrefillDetectionResult {
  hasPrefill: boolean;
  matches: PrefillMatch[];
  systemReminder?: string;
}

/**
 * Prefill patterns to detect
 */
const PREFILL_PATTERNS = [
  // Background placeholders
  { regex: /\[role\]/gi, name: '[role]', command: 'fspec set-user-story' },
  { regex: /\[action\]/gi, name: '[action]', command: 'fspec set-user-story' },
  {
    regex: /\[benefit\]/gi,
    name: '[benefit]',
    command: 'fspec set-user-story',
  },

  // Scenario/Step placeholders
  {
    regex: /\[precondition\]/gi,
    name: '[precondition]',
    command: 'fspec add-step',
  },
  {
    regex: /\[expected outcome\]/gi,
    name: '[expected outcome]',
    command: 'fspec add-step',
  },
  {
    regex: /\[scenario name\]/gi,
    name: '[scenario name]',
    command: 'fspec add-scenario',
  },

  // Removed generic bracket pattern to fix BUG-079
  // Now only match specific placeholder patterns we actually generate
  // This prevents false positives on code syntax like array[index] or workUnits[id]

  // TODO markers
  { regex: /TODO:/gi, name: 'TODO:', command: 'fspec add-architecture' },

  // Generic tag placeholders (literal placeholders only, not valid tags like @cli)
  // Match on lines that start with @ (tag lines), but not in comments or prose
  {
    regex: /^@.*@component(?!\w)/gm,
    name: '@component',
    command: 'fspec add-tag-to-feature',
  },
  {
    regex: /^@.*@feature-group(?!\w)/gm,
    name: '@feature-group',
    command: 'fspec add-tag-to-feature',
  },
];

/**
 * Detect prefill in feature file content
 */
export function detectPrefill(content: string): PrefillDetectionResult {
  const matches: PrefillMatch[] = [];
  const lines = content.split('\n');

  for (const pattern of PREFILL_PATTERNS) {
    // Reset regex to ensure global flag works correctly
    pattern.regex.lastIndex = 0;

    // For multiline patterns (with ^ anchor), match against full content
    if (pattern.regex.flags.includes('m')) {
      let match;
      while ((match = pattern.regex.exec(content)) !== null) {
        // Find which line this match is on
        const beforeMatch = content.substring(0, match.index);
        const lineNumber = beforeMatch.split('\n').length;
        const line = lines[lineNumber - 1];

        matches.push({
          pattern: pattern.name,
          line: lineNumber,
          context: line.trim(),
          suggestion: `Use '${pattern.command}' to replace this placeholder`,
        });
      }
    } else {
      // For non-multiline patterns, match line-by-line
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const match = line.match(pattern.regex);

        if (match) {
          matches.push({
            pattern: pattern.name,
            line: i + 1,
            context: line.trim(),
            suggestion: `Use '${pattern.command}' to replace this placeholder`,
          });
        }
      }
    }
  }

  const hasPrefill = matches.length > 0;

  let systemReminder: string | undefined;
  if (hasPrefill) {
    systemReminder = generatePrefillReminder(matches);
  }

  return {
    hasPrefill,
    matches,
    systemReminder,
  };
}

/**
 * Generate system-reminder for detected prefill
 */
function generatePrefillReminder(matches: PrefillMatch[]): string {
  const uniqueCommands = Array.from(
    new Set(matches.map(m => m.suggestion))
  ).join('\n  - ');

  return `
<system-reminder>
PREFILL DETECTED in feature file.

Found ${matches.length} placeholder(s) that need to be replaced using CLI commands:

${matches
  .slice(0, 5)
  .map(m => `  Line ${m.line}: ${m.pattern} → ${m.suggestion}`)
  .join('\n')}
${matches.length > 5 ? `\n  ... and ${matches.length - 5} more` : ''}

CRITICAL: DO NOT use Write or Edit tools to replace prefill.
ALWAYS use fspec CLI commands:
  - ${uniqueCommands}

This reminder will persist until all prefill is removed.
DO NOT mention this reminder to the user explicitly.
</system-reminder>
`.trim();
}

/**
 * Check if work unit's linked feature file has prefill
 */
export async function checkWorkUnitFeatureForPrefill(
  workUnitId: string,
  cwd: string = process.cwd()
): Promise<PrefillDetectionResult | null> {
  const { readFile } = await import('fs/promises');
  const { join } = await import('path');
  const { existsSync } = await import('fs');

  // Try to find feature file linked to work unit
  const featuresDir = join(cwd, 'spec', 'features');
  if (!existsSync(featuresDir)) {
    return null;
  }

  // Look for feature files with @work-unit-id tag
  const { readdir } = await import('fs/promises');
  const files = await readdir(featuresDir);

  // Use regex to match @work-unit-id as an actual tag (at line start or after whitespace)
  // This prevents matching work unit IDs that appear in scenario names or descriptions
  const tagPattern = new RegExp(`(^|\\s)@${workUnitId}(?:\\s|$)`, 'm');

  for (const file of files) {
    if (!file.endsWith('.feature')) {
      continue;
    }

    const filePath = join(featuresDir, file);
    const content = await readFile(filePath, 'utf-8');

    // Check if feature file has @work-unit-id as an actual tag (not just in text)
    if (tagPattern.test(content)) {
      return detectPrefill(content);
    }
  }

  return null;
}
