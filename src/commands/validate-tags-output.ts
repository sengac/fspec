/**
 * Output-rendering helpers for `fspec validate-tags`.
 *
 * The CLI printing branch is extracted here so that src/commands/validate-tags.ts
 * can stay under the project-wide 300-line file-size budget while still housing
 * the full validation logic.
 *
 * These helpers are intentionally pure over their inputs: they accept an array
 * of validation results plus the user's flag choices and emit lines through
 * the shared `output` abstraction. Programmatic callers of `validateTags()` are
 * unaffected — only the CLI printing path uses this module.
 */

import { output } from '../utils/output';

export interface TagValidationError {
  tag: string;
  message: string;
  suggestion?: string;
}

export interface TagValidationResultSummary {
  file: string;
  valid: boolean;
  errors: TagValidationError[];
}

export interface RenderValidateTagsOptions {
  verbose?: boolean;
  summary?: boolean;
}

export interface RenderValidateTagsInput {
  results: TagValidationResultSummary[];
  validCount: number;
  invalidCount: number;
  options: RenderValidateTagsOptions;
}

/**
 * Render the CLI output for a `validate-tags` invocation.
 *
 * Rules (from VAL-006):
 *   - Default (no flags): print ONLY ✗ violation blocks plus the summary
 *     counts. No per-file ✓ lines.
 *   - `--verbose`: also print one ✓ line per passing file.
 *   - `--summary`: suppress ALL per-file output and print only the summary
 *     count lines.
 *   - When both `--summary` and `--verbose` are passed, `--summary` wins
 *     (the quietest flag dominates).
 *   - The summary section is printed whenever there is more than one file
 *     under validation, OR whenever `--summary` is explicitly requested.
 *     For a single-file invocation with no flags and no failures, the
 *     command produces no output at all.
 */
export function renderValidateTagsOutput(input: RenderValidateTagsInput): void {
  const { results, validCount, invalidCount, options } = input;

  const summaryOnly = options.summary === true;
  const verbose = options.verbose === true && !summaryOnly;

  if (!summaryOnly) {
    for (const result of results) {
      if (result.valid) {
        if (verbose) {
          output.log(`✓ All tags in ${result.file} are registered`);
        }
      } else {
        output.log(`✗ ${result.file} has tag violations:`);
        for (const error of result.errors) {
          output.log(`  ${error.message}`);
          if (error.suggestion) {
            output.log(`  Suggestion: ${error.suggestion}`);
          }
        }
      }
    }
  }

  const shouldPrintSummary = summaryOnly || results.length > 1;
  if (!shouldPrintSummary) {
    return;
  }

  if (invalidCount === 0) {
    output.log(`✓ ${validCount} files passed`);
  } else {
    output.log(`✓ ${validCount} files passed`);
    output.log(`✗ ${invalidCount} files have tag violations`);
  }
}
