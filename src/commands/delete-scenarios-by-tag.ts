import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface DeleteScenariosByTagOptions {
  tags: string[];
  dryRun?: boolean;
  cwd?: string;
}

interface ScenarioInfo {
  file: string;
  name: string;
  tags: string[];
  lineStart: number;
  lineEnd: number;
}

interface DeleteScenariosByTagResult {
  success: boolean;
  deletedCount: number;
  fileCount: number;
  message?: string;
  scenarios?: ScenarioInfo[];
  error?: string;
}

export async function deleteScenariosByTag(
  options: DeleteScenariosByTagOptions
): Promise<DeleteScenariosByTagResult> {
  const { tags, dryRun = false, cwd = process.cwd() } = options;

  try {
    // Get all feature files
    const files = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    if (files.length === 0) {
      return {
        success: true,
        deletedCount: 0,
        fileCount: 0,
        message: 'No feature files found',
      };
    }

    // Find all scenarios matching the tags (AND logic)
    const matchingScenarios: Map<string, ScenarioInfo[]> = new Map();

    for (const file of files) {
      const filePath = join(cwd, file);
      const content = await readFile(filePath, 'utf-8');

      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      try {
        gherkinDocument = parser.parse(content);
      } catch {
        // Skip files with invalid syntax
        continue;
      }

      if (!gherkinDocument.feature) {
        continue;
      }

      const fileScenarios: ScenarioInfo[] = [];

      for (const child of gherkinDocument.feature.children) {
        if (!child.scenario) {
          continue;
        }

        const scenario = child.scenario;
        const scenarioTags = scenario.tags.map(t => t.name);

        // Check if scenario has ALL specified tags (AND logic)
        const hasAllTags = tags.every(tag => scenarioTags.includes(tag));

        if (hasAllTags) {
          // Find the line range for this scenario
          const scenarioLineStart = scenario.location.line;

          // Find scenario end by looking for next scenario or end of file
          const lines = content.split('\n');
          let scenarioLineEnd = lines.length;

          // Find the first tag of this scenario (which may be before the scenario keyword)
          const firstTagLine =
            scenario.tags.length > 0
              ? scenario.tags[0].location.line
              : scenarioLineStart;

          // Find next scenario or background to determine end
          for (const otherChild of gherkinDocument.feature.children) {
            if (otherChild.scenario && otherChild.scenario !== scenario) {
              const otherStartLine =
                otherChild.scenario.tags.length > 0
                  ? otherChild.scenario.tags[0].location.line
                  : otherChild.scenario.location.line;

              if (
                otherStartLine > scenarioLineStart &&
                otherStartLine < scenarioLineEnd
              ) {
                scenarioLineEnd = otherStartLine;
              }
            } else if (otherChild.background) {
              const bgStartLine = otherChild.background.location.line;
              if (
                bgStartLine > scenarioLineStart &&
                bgStartLine < scenarioLineEnd
              ) {
                scenarioLineEnd = bgStartLine;
              }
            }
          }

          fileScenarios.push({
            file,
            name: scenario.name,
            tags: scenarioTags,
            lineStart: firstTagLine,
            lineEnd: scenarioLineEnd,
          });
        }
      }

      if (fileScenarios.length > 0) {
        matchingScenarios.set(file, fileScenarios);
      }
    }

    // Count total scenarios to delete
    const totalScenarios = Array.from(matchingScenarios.values()).reduce(
      (sum, scenarios) => sum + scenarios.length,
      0
    );

    if (totalScenarios === 0) {
      return {
        success: true,
        deletedCount: 0,
        fileCount: 0,
        message: 'No scenarios found matching tags',
      };
    }

    // Dry run - just report what would happen
    if (dryRun) {
      const allScenarios = Array.from(matchingScenarios.values()).flat();
      return {
        success: true,
        deletedCount: totalScenarios,
        fileCount: matchingScenarios.size,
        message: `Would delete ${totalScenarios} scenario(s) from ${matchingScenarios.size} file(s)`,
        scenarios: allScenarios,
      };
    }

    // Perform deletions
    let filesModified = 0;

    for (const [file, scenarios] of matchingScenarios.entries()) {
      const filePath = join(cwd, file);
      const content = await readFile(filePath, 'utf-8');
      const lines = content.split('\n');

      // Sort scenarios by line number descending so we can delete from bottom up
      const sortedScenarios = [...scenarios].sort(
        (a, b) => b.lineStart - a.lineStart
      );

      // Remove each scenario
      for (const scenario of sortedScenarios) {
        // Remove lines from lineStart to lineEnd (exclusive)
        const startIndex = scenario.lineStart - 1;
        const endIndex = scenario.lineEnd - 1;

        lines.splice(startIndex, endIndex - startIndex);
      }

      // Join back and clean up excessive blank lines
      let newContent = lines.join('\n');

      // Remove excessive blank lines (more than 2 consecutive)
      newContent = newContent.replace(/\n{4,}/g, '\n\n\n');

      // Validate the new content is valid Gherkin
      const newBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const newMatcher = new Gherkin.GherkinClassicTokenMatcher();
      const newParser = new Gherkin.Parser(newBuilder, newMatcher);

      try {
        newParser.parse(newContent);
      } catch (error: any) {
        return {
          success: false,
          deletedCount: 0,
          fileCount: 0,
          error: `Validation failed after deleting scenarios from ${file}: ${error.message}`,
        };
      }

      // Write the modified content
      await writeFile(filePath, newContent, 'utf-8');
      filesModified++;

      // Update coverage file to remove deleted scenarios
      const coverageFilePath = `${filePath}.coverage`;
      try {
        const coverageContent = await readFile(coverageFilePath, 'utf-8');
        const coverage = JSON.parse(coverageContent);

        // Get deleted scenario names for this file
        const deletedNames = new Set(scenarios.map(s => s.name));

        // Remove deleted scenarios from coverage
        coverage.scenarios = coverage.scenarios.filter(
          (s: any) => !deletedNames.has(s.name)
        );

        // Recalculate stats
        const coveredScenarios = coverage.scenarios.filter(
          (s: any) => s.testMappings && s.testMappings.length > 0
        ).length;

        coverage.stats = {
          ...coverage.stats,
          totalScenarios: coverage.scenarios.length,
          coveredScenarios,
          coveragePercent:
            coverage.scenarios.length > 0
              ? Math.round((coveredScenarios / coverage.scenarios.length) * 100)
              : 0,
        };

        // Write updated coverage
        await writeFile(
          coverageFilePath,
          JSON.stringify(coverage, null, 2),
          'utf-8'
        );
      } catch (error: any) {
        // Coverage file doesn't exist or invalid - skip cleanup but continue
      }
    }

    return {
      success: true,
      deletedCount: totalScenarios,
      fileCount: filesModified,
      message: `Deleted ${totalScenarios} scenario(s) from ${filesModified} file(s). All modified files validated successfully.`,
    };
  } catch (error: any) {
    return {
      success: false,
      deletedCount: 0,
      fileCount: 0,
      error: error.message,
    };
  }
}

export async function deleteScenariosByTagCommand(options: {
  tag?: string | string[];
  dryRun?: boolean;
}): Promise<void> {
  try {
    // Normalize tag input to array
    let tags: string[];
    if (Array.isArray(options.tag)) {
      tags = options.tag;
    } else if (options.tag) {
      tags = [options.tag];
    } else {
      console.error(chalk.red('Error:'), 'At least one --tag is required');
      process.exit(1);
    }

    const result = await deleteScenariosByTag({
      tags,
      dryRun: options.dryRun,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (options.dryRun && result.scenarios) {
      console.log(chalk.yellow('Dry run mode - no files modified'));
      console.log(
        chalk.cyan(
          `\nWould delete ${result.deletedCount} scenario(s) from ${result.fileCount} file(s):\n`
        )
      );

      // Group scenarios by file
      const byFile = new Map<string, ScenarioInfo[]>();
      for (const scenario of result.scenarios) {
        if (!byFile.has(scenario.file)) {
          byFile.set(scenario.file, []);
        }
        byFile.get(scenario.file)!.push(scenario);
      }

      for (const [file, scenarios] of byFile.entries()) {
        console.log(chalk.white(`\n${file}:`));
        for (const scenario of scenarios) {
          console.log(
            chalk.gray(`  - ${scenario.name} (${scenario.tags.join(' ')})`)
          );
        }
      }
    } else {
      console.log(chalk.green(`✓ ${result.message}`));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerDeleteScenariosCommand(program: Command): void {
  program
    .command('delete-scenarios')
    .description('Bulk delete scenarios by tag across multiple files')
    .option(
      '--tag <tag>',
      'Filter by tag (can specify multiple times for AND logic)',
      (value, previous) => {
        return previous ? [...previous, value] : [value];
      }
    )
    .option('--dry-run', 'Preview deletions without making changes')
    .action(deleteScenariosByTagCommand);
}
