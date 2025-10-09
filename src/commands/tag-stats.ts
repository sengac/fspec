import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

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

  // Load tag registry from TAGS.md (optional)
  let registry: Map<string, string> | null = null;
  let tagsFileFound = true;
  try {
    registry = await loadTagRegistry(cwd);
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

  if (registry) {
    // Group by categories from TAGS.md
    const categoryMap = await loadCategoryMap(cwd);

    for (const [categoryName, categoryTags] of categoryMap.entries()) {
      const tagStats: TagCount[] = [];

      for (const tag of categoryTags) {
        registeredTags.add(tag);
        const count = tagCounts.get(tag) || 0;
        if (count > 0) {
          tagStats.push({ tag, count });
        }
      }

      // Sort by count descending
      tagStats.sort((a, b) => b.count - a.count);

      if (tagStats.length > 0) {
        categories.push({ name: categoryName, tags: tagStats });
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
    // No TAGS.md - all tags are unregistered
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
  if (registry) {
    for (const tag of registeredTags) {
      if (!tagCounts.has(tag)) {
        unusedTags.push(tag);
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
      console.log(chalk.yellow('\n⚠ Warning: spec/TAGS.md not found'));
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

async function loadTagRegistry(cwd: string): Promise<Map<string, string>> {
  const tagsPath = join(cwd, 'spec', 'TAGS.md');
  const content = await readFile(tagsPath, 'utf-8');

  const tagPattern = /\|\s*`(@[a-z0-9-]+)`\s*\|/g;
  const tags = new Map<string, string>();

  let match;
  while ((match = tagPattern.exec(content)) !== null) {
    tags.set(match[1], match[1]);
  }

  return tags;
}

async function loadCategoryMap(cwd: string): Promise<Map<string, string[]>> {
  const tagsPath = join(cwd, 'spec', 'TAGS.md');
  const content = await readFile(tagsPath, 'utf-8');

  const categoryMap = new Map<string, string[]>();

  // Match category headers: ## Category Name
  const categoryPattern = /^## (.+)$/gm;
  let categoryMatch;

  while ((categoryMatch = categoryPattern.exec(content)) !== null) {
    const categoryName = categoryMatch[1].trim();
    const categoryStart = categoryMatch.index;

    // Find the next category or end of file
    const nextCategoryMatch = content
      .substring(categoryStart + 1)
      .match(/^## /m);
    const categoryEnd = nextCategoryMatch
      ? categoryStart + 1 + nextCategoryMatch.index
      : content.length;

    const categoryContent = content.substring(categoryStart, categoryEnd);

    // Extract tags from this category
    const tags: string[] = [];
    const tagPattern = /\|\s*`(@[a-z0-9-]+)`\s*\|/g;
    let tagMatch;

    while ((tagMatch = tagPattern.exec(categoryContent)) !== null) {
      tags.push(tagMatch[1]);
    }

    if (tags.length > 0) {
      categoryMap.set(categoryName, tags);
    }
  }

  return categoryMap;
}
