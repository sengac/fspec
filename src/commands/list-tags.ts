import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

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

export async function listTags(options: ListTagsOptions = {}): Promise<ListTagsResult> {
  const cwd = options.cwd || process.cwd();
  const tagsPath = join(cwd, 'spec', 'TAGS.md');

  // Read TAGS.md
  let content: string;
  try {
    content = await readFile(tagsPath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new Error('TAGS.md not found: spec/TAGS.md');
    }
    throw error;
  }

  // Parse categories and tags
  const categories = parseTagsFromContent(content);

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

function parseTagsFromContent(content: string): CategoryEntry[] {
  const categories: CategoryEntry[] = [];

  // Match category headers: ## Category Name
  const categoryPattern = /^## (.+)$/gm;
  let categoryMatch;

  while ((categoryMatch = categoryPattern.exec(content)) !== null) {
    const categoryName = categoryMatch[1].trim();
    const categoryStart = categoryMatch.index;

    // Find the next category or end of file
    const nextCategoryMatch = content.substring(categoryStart + 1).match(/^## /m);
    const categoryEnd = nextCategoryMatch
      ? categoryStart + 1 + nextCategoryMatch.index
      : content.length;

    const categoryContent = content.substring(categoryStart, categoryEnd);

    // Extract tags from this category
    const tags: TagEntry[] = [];
    const tagPattern = /\|\s*`(@[a-z0-9-]+)`\s*\|\s*(.+?)\s*\|/g;
    let tagMatch;

    while ((tagMatch = tagPattern.exec(categoryContent)) !== null) {
      tags.push({
        tag: tagMatch[1],
        description: tagMatch[2].trim(),
      });
    }

    // Sort tags alphabetically
    tags.sort((a, b) => a.tag.localeCompare(b.tag));

    categories.push({
      name: categoryName,
      tags,
    });
  }

  return categories;
}

export async function listTagsCommand(options: { category?: string } = {}): Promise<void> {
  try {
    const result = await listTags({ category: options.category });

    // Display results
    for (const category of result.categories) {
      console.log(chalk.bold.blue(`\n${category.name}`) + chalk.gray(` (${category.tags.length} tags)`));

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
    if (error.message.includes('TAGS.md not found')) {
      console.error(chalk.red(error.message));
      console.log(chalk.yellow('  Suggestion: Create spec/TAGS.md or use "fspec register-tag" to add tags'));
      process.exit(2);
    }
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
