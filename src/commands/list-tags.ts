import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Tags } from '../types/tags';

interface TagEntry {
  tag: string;
  description: string;
}

interface CategoryEntry {
  name: string;
  tags: TagEntry[];
}

interface ListTagsOptions {
  category?: string;
  cwd?: string;
}

interface ListTagsResult {
  success: boolean;
  categories: CategoryEntry[];
}

export async function listTags(
  options: ListTagsOptions = {}
): Promise<ListTagsResult> {
  const cwd = options.cwd || process.cwd();
  const tagsJsonPath = join(cwd, 'spec', 'tags.json');

  // Check if tags.json exists
  if (!existsSync(tagsJsonPath)) {
    throw new Error('tags.json not found: spec/tags.json');
  }

  // Read tags.json
  let content: string;
  try {
    content = await readFile(tagsJsonPath, 'utf-8');
  } catch (error: any) {
    throw new Error(`Failed to read tags.json: ${error.message}`);
  }

  // Parse JSON
  let tagsData: Tags;
  try {
    tagsData = JSON.parse(content);
  } catch (error: any) {
    throw new Error(`Failed to parse tags.json: ${error.message}`);
  }

  // Transform to CategoryEntry format
  const categories: CategoryEntry[] = tagsData.categories.map(cat => ({
    name: cat.name,
    tags: cat.tags
      .map(t => ({
        tag: t.name,
        description: t.description,
      }))
      .sort((a, b) => a.tag.localeCompare(b.tag)), // Sort alphabetically
  }));

  // Filter by category if specified
  if (options.category) {
    const filtered = categories.filter(c => c.name === options.category);
    if (filtered.length === 0) {
      const availableCategories = categories.map(c => c.name).join(', ');
      throw new Error(
        `Category not found: ${options.category}. Available categories: ${availableCategories}`
      );
    }
    return { success: true, categories: filtered };
  }

  return { success: true, categories };
}

export async function listTagsCommand(
  options: { category?: string } = {}
): Promise<void> {
  try {
    const result = await listTags({ category: options.category });

    // Display results
    for (const category of result.categories) {
      console.log(
        chalk.bold.blue(`\n${category.name}`) +
          chalk.gray(` (${category.tags.length} tags)`)
      );

      if (category.tags.length === 0) {
        console.log(chalk.gray('  No tags registered'));
      } else {
        for (const tag of category.tags) {
          console.log(`  ${chalk.green(tag.tag)} - ${tag.description}`);
        }
      }
    }

    console.log('');
    process.exit(0);
  } catch (error: any) {
    if (error.message.includes('tags.json not found')) {
      console.error(chalk.red(error.message));
      console.log(
        chalk.yellow(
          '  Suggestion: Create spec/tags.json or use "fspec register-tag" to add tags'
        )
      );
      process.exit(2);
    }
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
