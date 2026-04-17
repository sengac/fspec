import type { Command } from 'commander';
import { glob } from 'tinyglobby';
import { loadWorkUnitsData } from '../utils/work-unit-tags';

import { output } from '../utils/output';
import { renderValidateTagsOutput } from './validate-tags-output';
import { loadTagRegistry } from './validate-tags-registry';
import { validateFileTags } from './validate-tags-file';
import type { TagValidationResult } from './validate-tags-file';

/**
 * Run tag validation across one or all feature files. The programmatic API
 * used by the CLI command and tests — returns aggregate counts plus the
 * per-file results without mutating process state.
 *
 * @param options.file - Validate only this feature file (optional).
 * @param options.cwd - Project root (defaults to `process.cwd()`).
 * @returns Aggregate validation outcome.
 */
export async function validateTags(
  options: { file?: string; cwd?: string } = {}
): Promise<{
  results: TagValidationResult[];
  validCount: number;
  invalidCount: number;
}> {
  const cwd = options.cwd || process.cwd();

  const registry = await loadTagRegistry(cwd);
  const workUnitsData = await loadWorkUnitsData(cwd);

  const files = options.file
    ? [options.file]
    : await glob(['spec/features/**/*.feature'], { cwd, absolute: false });

  if (files.length === 0) {
    return { results: [], validCount: 0, invalidCount: 0 };
  }

  const results = await Promise.all(
    files.map(file => validateFileTags(file, registry, workUnitsData, cwd))
  );

  const validCount = results.filter(r => r.valid).length;
  const invalidCount = results.length - validCount;

  return { results, validCount, invalidCount };
}

/**
 * Options for the `fspec validate-tags` CLI command. Mirrors the flags
 * declared in `registerValidateTagsCommand`.
 */
export interface ValidateTagsCommandOptions {
  verbose?: boolean;
  summary?: boolean;
}

/**
 * CLI entry point for `fspec validate-tags`. Runs validation, renders the
 * appropriate output mode (failures-only, verbose, or summary), and exits
 * with a status code reflecting the outcome.
 *
 * Exit codes:
 *   0 — all files valid
 *   1 — one or more files failed validation
 *   2 — unexpected error while running validation
 *
 * @param file - Optional single feature file to validate.
 * @param options - Output mode flags.
 */
export async function validateTagsCommand(
  file?: string,
  options: ValidateTagsCommandOptions = {}
): Promise<void> {
  try {
    const { results, validCount, invalidCount } = await validateTags({ file });

    renderValidateTagsOutput({ results, validCount, invalidCount, options });

    if (invalidCount > 0) {
      process.exit(1);
    } else {
      process.exit(0);
    }
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    output.error('Error:', message);
    process.exit(2);
  }
}

/**
 * Register the `validate-tags` subcommand on the given Commander program.
 * Wires the CLI flags to {@link validateTagsCommand}.
 *
 * @param program - Root commander program to attach the subcommand to.
 */
export function registerValidateTagsCommand(program: Command): void {
  program
    .command('validate-tags')
    .description(
      'Validate feature file tags against TAGS.md registry. Default output shows only failures plus a summary; use --verbose to also print one ✓ line per passing file, or --summary to print only the summary count lines.'
    )
    .argument(
      '[file]',
      'Feature file to validate (validates all if not specified)'
    )
    .option(
      '--verbose',
      'Print one ✓ line per passing file (default: failures-only)'
    )
    .option(
      '--summary',
      'Print only the summary count lines (no per-file output). Overrides --verbose.'
    )
    .action(
      async (file: string | undefined, options: ValidateTagsCommandOptions) => {
        await validateTagsCommand(file, options);
      }
    );
}
