import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Tags } from '../types/tags';
import { validateTagsJson } from '../validators/json-schema';
import { generateTagsMd } from '../generators/tags-md';

interface UpdateTagOptions {
  tag: string;
  category?: string;
  description?: string;
  cwd?: string;
}

interface UpdateTagResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function updateTag(
  options: UpdateTagOptions
): Promise<UpdateTagResult> {
  const { tag, category, description, cwd = process.cwd() } = options;
  const tagsJsonPath = join(cwd, 'spec', 'tags.json');
  const tagsMdPath = join(cwd, 'spec', 'TAGS.md');

  // Validate that at least one update is specified
  if (!category && !description) {
    return {
      success: false,
      error: 'No updates specified. Use --category and/or --description',
    };
  }

  // Check if tags.json exists
  if (!existsSync(tagsJsonPath)) {
    return {
      success: false,
      error: 'spec/tags.json not found',
    };
  }

  try {
    // Read tags.json
    const content = await readFile(tagsJsonPath, 'utf-8');
    const tagsData: Tags = JSON.parse(content);

    // Find the tag in categories
    let currentCategory: any = null;
    let tagIndex = -1;

    for (const cat of tagsData.categories) {
      const idx = cat.tags.findIndex((t) => t.name === tag);
      if (idx !== -1) {
        currentCategory = cat;
        tagIndex = idx;
        break;
      }
    }

    if (!currentCategory) {
      return {
        success: false,
        error: `Tag ${tag} not found in registry`,
      };
    }

    const currentTag = currentCategory.tags[tagIndex];

    // If category is being changed, validate new category exists
    if (category && category !== currentCategory.name) {
      const targetCategory = tagsData.categories.find(
        (c) => c.name === category
      );

      if (!targetCategory) {
        const availableCategories = tagsData.categories.map((c) => c.name);
        return {
          success: false,
          error: `Invalid category: ${category}. Available categories: ${availableCategories.join(', ')}`,
        };
      }

      // Remove tag from current category
      currentCategory.tags.splice(tagIndex, 1);

      // Add tag to new category with updated description (or keep existing)
      targetCategory.tags.push({
        name: tag,
        description: description || currentTag.description,
      });

      // Sort tags alphabetically within new category
      targetCategory.tags.sort((a, b) => a.name.localeCompare(b.name));
    } else {
      // Update description only (tag stays in same category)
      if (description) {
        currentTag.description = description;
      }
    }

    // Write updated tags.json
    await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2), 'utf-8');

    // Validate updated JSON against schema
    const validation = await validateTagsJson(tagsJsonPath);
    if (!validation.valid) {
      return {
        success: false,
        error: `Updated tags.json failed schema validation: ${validation.errors?.join(', ')}`,
      };
    }

    // Regenerate TAGS.md from JSON
    const markdown = await generateTagsMd(tagsData);
    await writeFile(tagsMdPath, markdown, 'utf-8');

    return {
      success: true,
      message: `Successfully updated ${tag}`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function updateTagCommand(options: {
  tag: string;
  category?: string;
  description?: string;
}): Promise<void> {
  try {
    const result = await updateTag(options);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
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
