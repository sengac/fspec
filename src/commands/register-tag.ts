import { writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { Tags, TagCategory } from '../types/tags';
import { validateTagsJson } from '../validators/json-schema';
import { generateTagsMd } from '../generators/tags-md';
import { ensureTagsFile } from '../utils/ensure-files';

interface RegisterTagOptions {
  cwd?: string;
}

interface RegisterTagResult {
  success: boolean;
  message: string;
  created?: boolean;
  converted?: boolean;
}

export async function registerTag(
  tag: string,
  category: string,
  description: string,
  options: RegisterTagOptions = {}
): Promise<RegisterTagResult> {
  const cwd = options.cwd || process.cwd();
  const tagsJsonPath = join(cwd, 'spec', 'tags.json');
  const tagsMdPath = join(cwd, 'spec', 'TAGS.md');

  // Validate and normalize tag format
  let normalizedTag = tag;
  let converted = false;

  if (!tag.startsWith('@')) {
    throw new Error(
      `Invalid tag format: "${tag}". Valid format is @lowercase-with-hyphens`
    );
  }

  // Convert to lowercase if needed
  if (tag !== tag.toLowerCase()) {
    normalizedTag = tag.toLowerCase();
    converted = true;
  }

  // Validate format
  if (!/^@[a-z0-9-]+$/.test(normalizedTag)) {
    throw new Error(
      `Invalid tag format: "${tag}". Valid format is @lowercase-with-hyphens`
    );
  }

  // Load or create tags.json using ensureTagsFile
  const tagsData: Tags = await ensureTagsFile(cwd);
  const created = false; // ensureTagsFile handles creation

  // Check for duplicate tags across all categories
  for (const cat of tagsData.categories) {
    const existingTag = cat.tags.find(t => t.name === normalizedTag);
    if (existingTag) {
      throw new Error(
        `Tag ${normalizedTag} is already registered in ${cat.name}`
      );
    }
  }

  // Find target category (case-insensitive)
  const targetCategory = tagsData.categories.find(
    c => c.name.toLowerCase() === category.toLowerCase()
  );

  if (!targetCategory) {
    const availableCategories = tagsData.categories.map(c => c.name);
    throw new Error(
      `Invalid category: "${category}". Available categories: ${availableCategories.join(', ')}`
    );
  }

  // Add tag to category in alphabetical order
  const newTag = {
    name: normalizedTag,
    description,
  };

  targetCategory.tags.push(newTag);

  // Sort tags alphabetically within category
  targetCategory.tags.sort((a, b) => a.name.localeCompare(b.name));

  // Update statistics
  tagsData.statistics.lastUpdated = new Date().toISOString();

  // Save original tags.json for rollback
  const originalTagsData = await ensureTagsFile(cwd);

  // Write updated tags.json
  await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2), 'utf-8');

  // Validate updated JSON against schema
  const validation = await validateTagsJson(tagsJsonPath);
  if (!validation.valid) {
    throw new Error(
      `Updated tags.json failed schema validation: ${validation.errors?.join(', ')}`
    );
  }

  // Regenerate TAGS.md from JSON with rollback on failure
  try {
    const markdown = await generateTagsMd(tagsData);
    await writeFile(tagsMdPath, markdown, 'utf-8');
  } catch (error: any) {
    // Rollback tags.json to previous state
    await writeFile(
      tagsJsonPath,
      JSON.stringify(originalTagsData, null, 2),
      'utf-8'
    );
    throw new Error(
      `Failed to regenerate TAGS.md - changes rolled back: ${error.message}`
    );
  }

  const message = converted
    ? `Successfully registered ${normalizedTag} (converted from ${tag}) in ${category}`
    : `Successfully registered ${normalizedTag} in ${category}`;

  return {
    success: true,
    message,
    created,
    converted,
  };
}

export async function registerTagCommand(
  tag: string,
  category: string,
  description: string
): Promise<void> {
  try {
    const result = await registerTag(tag, category, description);

    if (result.created) {
      console.log(chalk.yellow('Created new tags.json and TAGS.md'));
    }

    if (result.converted) {
      console.log(
        chalk.yellow(
          `Note: Tag converted to lowercase: ${tag} → ${tag.toLowerCase()}`
        )
      );
    }

    console.log(chalk.green(`✓ ${result.message}`));
    console.log(chalk.gray('  Updated: spec/tags.json'));
    console.log(chalk.gray('  Regenerated: spec/TAGS.md'));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerRegisterTagCommand(program: Command): void {
  program
    .command('register-tag')
    .description('Register a new tag in TAGS.md registry')
    .argument('<tag>', 'Tag name (e.g., "@my-tag")')
    .argument('<category>', 'Category name (e.g., "Technical Tags")')
    .argument('<description>', 'Tag description')
    .action(registerTagCommand);
}
