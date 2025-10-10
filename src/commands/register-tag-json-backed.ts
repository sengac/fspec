import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { TagsRegistry } from '../types/tags';
import { validateTagsJson as validateTagsJsonFile } from '../validators/json-schema';
import { generateTagsMd } from '../generators/tags-md';

interface Tag {
  name: string;
  description: string;
  usage?: string;
  scope?: string;
  examples?: string;
  useCases?: string;
  whenToUse?: string;
  criteria?: string;
  meaning?: string;
  testType?: string;
}

interface RegisterTagResult {
  success: boolean;
}

const TAG_NAME_REGEX = /^@[a-z0-9-]+$/;

export async function registerTagJsonBacked(options: {
  tagName: string;
  category: string;
  description: string;
  usage?: string;
  scope?: string;
  examples?: string;
  useCases?: string;
  whenToUse?: string;
  criteria?: string;
  meaning?: string;
  testType?: string;
  cwd?: string;
  forceRegenerationFailure?: boolean;
}): Promise<RegisterTagResult> {
  const cwd = options.cwd || process.cwd();
  const tagsFile = join(cwd, 'spec', 'tags.json');
  const tagsMdFile = join(cwd, 'spec', 'TAGS.md');

  try {
    // Validate tag name format
    if (!TAG_NAME_REGEX.test(options.tagName)) {
      throw new Error('Invalid tag name: must start with @ and contain only lowercase letters, numbers, and hyphens');
    }

    // Read tags.json
    const content = await readFile(tagsFile, 'utf-8');
    const originalContent = content;
    const data: TagsRegistry = JSON.parse(content);

    // Find category
    const category = data.categories.find(c => c.name === options.category);
    if (!category) {
      throw new Error(`Category '${options.category}' not found`);
    }

    // Check if tag already exists (in any category)
    for (const cat of data.categories) {
      if (cat.tags.some(t => t.name === options.tagName)) {
        throw new Error(`Tag ${options.tagName} already exists in registry`);
      }
    }

    // Create new tag
    const newTag: Tag = {
      name: options.tagName,
      description: options.description,
    };

    // Add optional fields based on category-specific properties
    if (options.usage) {
      newTag.usage = options.usage;
    }
    if (options.scope) {
      newTag.scope = options.scope;
    }
    if (options.examples) {
      newTag.examples = options.examples;
    }
    if (options.useCases) {
      newTag.useCases = options.useCases;
    }
    if (options.whenToUse) {
      newTag.whenToUse = options.whenToUse;
    }
    if (options.criteria) {
      newTag.criteria = options.criteria;
    }
    if (options.meaning) {
      newTag.meaning = options.meaning;
    }
    if (options.testType) {
      newTag.testType = options.testType;
    }

    // Add tag to category
    category.tags.push(newTag);

    // Update statistics timestamp
    data.statistics.lastUpdated = new Date().toISOString();

    // Write updated tags.json
    await writeFile(tagsFile, JSON.stringify(data, null, 2));

    // Validate updated tags.json against schema
    const validation = await validateTagsJsonFile(tagsFile);
    if (!validation.valid) {
      // Rollback on validation failure
      await writeFile(tagsFile, originalContent);
      throw new Error(`tags.json validation failed: ${validation.errors?.join(', ')}`);
    }

    // Regenerate TAGS.md
    try {
      if (options.forceRegenerationFailure) {
        throw new Error('Forced regeneration failure for testing');
      }

      const markdown = await generateTagsMd(data);
      await writeFile(tagsMdFile, markdown, 'utf-8');
    } catch (error: unknown) {
      // Rollback tags.json on markdown generation failure
      await writeFile(tagsFile, originalContent);
      if (error instanceof Error) {
        throw new Error(`Failed to regenerate TAGS.md - changes rolled back: ${error.message}`);
      }
      throw new Error('Failed to regenerate TAGS.md - changes rolled back');
    }

    return {
      success: true,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to register tag: ${error.message}`);
    }
    throw error;
  }
}
