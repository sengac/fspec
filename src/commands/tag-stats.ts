import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import type { Tags } from '../types/tags';

interface TagCount {
  tag: string;
  count: number;
}

interface CategoryStats {
  name: string;
  tags: TagCount[];
}

interface TagStatsOptions {
  cwd?: string;
}

interface TagStatsResult {
  success: boolean;
  totalFiles: number;
  uniqueTags: number;
  totalOccurrences: number;
  categories: CategoryStats[];
  unusedTags: string[];
  tagsFileFound: boolean;
  invalidFiles: string[];
}

export async function tagStats(
  options: TagStatsOptions = {}
): Promise<TagStatsResult> {
  const cwd = options.cwd || process.cwd();

  // Load tag registry from tags.json (optional)
  let tagsData: Tags | null = null;
  let tagsFileFound = true;
  try {
    tagsData = await loadTagsJson(cwd);
  } catch {
    tagsFileFound = false;
  }

  // Get all feature files
  const files = await glob(['spec/features/**/*.feature'], {
    cwd,
    absolute: false,
  });

  if (files.length === 0) {
    return {
      success: true,
      totalFiles: 0,
      uniqueTags: 0,
      totalOccurrences: 0,
      categories: [],
      unusedTags: [],
      tagsFileFound,
      invalidFiles: [],
    };
  }

  // Extract tags from all feature files
  const tagCounts = new Map<string, number>();
  const invalidFiles: string[] = [];

  for (const file of files) {
    try {
      const content = await readFile(join(cwd, file), 'utf-8');

      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      try {
        gherkinDocument = parser.parse(content);
      } catch {
        invalidFiles.push(file);
        continue;
      }

      if (!gherkinDocument.feature) {
        continue;
      }

      // Count tags from this feature
      const tags = gherkinDocument.feature.tags.map(t => t.name);
      for (const tag of tags) {
        tagCounts.set(tag, (tagCounts.get(tag) || 0) + 1);
      }
    } catch {
      invalidFiles.push(file);
    }
  }

  const uniqueTags = tagCounts.size;
  const totalOccurrences = Array.from(tagCounts.values()).reduce(
    (sum, count) => sum + count,
    0
  );

  // Build statistics by category
  const categories: CategoryStats[] = [];
  const registeredTags = new Set<string>();

  if (tagsData) {
    // Group by categories from tags.json
    for (const category of tagsData.categories) {
      const tagStats: TagCount[] = [];

      for (const tagDef of category.tags) {
        const tag = tagDef.name;
        registeredTags.add(tag);
        const count = tagCounts.get(tag) || 0;
        if (count > 0) {
          tagStats.push({ tag, count });
        }
      }

      // Sort by count descending
      tagStats.sort((a, b) => b.count - a.count);

      if (tagStats.length > 0) {
        categories.push({ name: category.name, tags: tagStats });
      }
    }

    // Find unregistered tags
    const unregisteredTags: TagCount[] = [];
    for (const [tag, count] of tagCounts.entries()) {
      if (!registeredTags.has(tag)) {
        unregisteredTags.push({ tag, count });
      }
    }

    if (unregisteredTags.length > 0) {
      unregisteredTags.sort((a, b) => b.count - a.count);
      categories.push({ name: 'Unregistered', tags: unregisteredTags });
    }
  } else {
    // No tags.json - all tags are unregistered
    const allTags: TagCount[] = Array.from(tagCounts.entries()).map(
      ([tag, count]) => ({
        tag,
        count,
      })
    );
    allTags.sort((a, b) => b.count - a.count);
    categories.push({ name: 'Unregistered', tags: allTags });
  }

  // Find unused registered tags
  const unusedTags: string[] = [];
  if (tagsData) {
    for (const category of tagsData.categories) {
      for (const tagDef of category.tags) {
        const tag = tagDef.name;
        registeredTags.add(tag);
        if (!tagCounts.has(tag)) {
          unusedTags.push(tag);
        }
      }
    }
    unusedTags.sort();
  }

  return {
    success: true,
    totalFiles: files.length,
    uniqueTags,
    totalOccurrences,
    categories,
    unusedTags,
    tagsFileFound,
    invalidFiles,
  };
}

export async function tagStatsCommand(): Promise<void> {
  try {
    const result = await tagStats();

    // Display overall statistics
    console.log(chalk.bold.blue('\nTag Usage Statistics'));
    console.log(chalk.gray('─'.repeat(50)));
    console.log(`Total feature files: ${chalk.cyan(result.totalFiles)}`);
    console.log(`Unique tags used: ${chalk.cyan(result.uniqueTags)}`);
    console.log(
      `Total tag occurrences: ${chalk.cyan(result.totalOccurrences)}`
    );

    if (!result.tagsFileFound) {
      console.log(chalk.yellow('\n⚠ Warning: spec/tags.json not found'));
    }

    if (result.invalidFiles.length > 0) {
      console.log(
        chalk.yellow(
          `\n⚠ Warning: ${result.invalidFiles.length} file(s) with invalid syntax skipped:`
        )
      );
      for (const file of result.invalidFiles) {
        console.log(chalk.yellow(`  - ${file}`));
      }
    }

    // Display per-category statistics
    if (result.categories.length > 0) {
      console.log(chalk.bold.blue('\n\nTag Counts by Category'));
      console.log(chalk.gray('─'.repeat(50)));

      for (const category of result.categories) {
        console.log(
          chalk.bold(`\n${category.name}`) +
            chalk.gray(` (${category.tags.length} tags)`)
        );
        for (const tagCount of category.tags) {
          console.log(
            `  ${chalk.green(tagCount.tag.padEnd(30))} ${chalk.cyan(tagCount.count)}`
          );
        }
      }
    }

    // Display unused tags
    if (result.unusedTags.length > 0) {
      console.log(chalk.bold.blue('\n\nUnused Registered Tags'));
      console.log(chalk.gray('─'.repeat(50)));
      console.log(
        chalk.yellow(
          `${result.unusedTags.length} registered tag(s) not used in any feature file:\n`
        )
      );
      for (const tag of result.unusedTags) {
        console.log(chalk.yellow(`  ${tag}`));
      }
    }

    console.log('');
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(2);
  }
}

async function loadTagsJson(cwd: string): Promise<Tags> {
  const tagsJsonPath = join(cwd, 'spec', 'tags.json');

  if (!existsSync(tagsJsonPath)) {
    throw new Error('tags.json not found: spec/tags.json');
  }

  const content = await readFile(tagsJsonPath, 'utf-8');
  const tagsData: Tags = JSON.parse(content);

  return tagsData;
}

export function registerTagStatsCommand(program: Command): void {
  program
    .command('tag-stats')
    .description('Show tag usage statistics across all feature files')
    .action(tagStatsCommand);
}
