import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Tags, TagCategory } from '../types/tags';
import { validateTagsJson } from '../validators/json-schema';
import { generateTagsMd } from '../generators/tags-md';

interface RegisterTagOptions {
  cwd?: string;
}

interface RegisterTagResult {
  success: boolean;
  message: string;
  created?: boolean;
  converted?: boolean;
}

// Minimal tags.json template for creation
const TAGS_JSON_TEMPLATE: Tags = {
  $schema: '../src/schemas/tags.schema.json',
  categories: [
    { name: 'Phase Tags', description: 'Phase identification tags', required: true, tags: [] },
    { name: 'Component Tags', description: 'Architectural component tags', required: true, tags: [] },
    { name: 'Feature Group Tags', description: 'Functional area tags', required: true, tags: [] },
    { name: 'Technical Tags', description: 'Technical concern tags', required: false, tags: [] },
    { name: 'Platform Tags', description: 'Platform-specific tags', required: false, tags: [] },
    { name: 'Priority Tags', description: 'Implementation priority tags', required: false, tags: [] },
    { name: 'Status Tags', description: 'Development status tags', required: false, tags: [] },
    { name: 'Testing Tags', description: 'Test-related tags', required: false, tags: [] },
    { name: 'CAGE Integration Tags', description: 'CAGE-specific tags', required: false, tags: [] },
  ],
  combinationExamples: [],
  usageGuidelines: {
    requiredCombinations: { title: '', requirements: [], minimumExample: '' },
    recommendedCombinations: { title: '', includes: [], recommendedExample: '' },
    orderingConvention: { title: '', order: [], example: '' },
  },
  addingNewTags: {
    process: [],
    namingConventions: [],
    antiPatterns: { dont: [], do: [] },
  },
  queries: { title: '', examples: [] },
  statistics: {
    lastUpdated: new Date().toISOString(),
    phaseStats: [],
    componentStats: [],
    featureGroupStats: [],
    updateCommand: 'fspec tag-stats',
  },
  validation: { rules: [], commands: [] },
  references: [],
};

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

  // Load or create tags.json
  let tagsData: Tags;
  let created = false;

  if (existsSync(tagsJsonPath)) {
    const content = await readFile(tagsJsonPath, 'utf-8');
    tagsData = JSON.parse(content);
  } else {
    // Create spec directory and tags.json from template
    await mkdir(join(cwd, 'spec'), { recursive: true });
    tagsData = JSON.parse(JSON.stringify(TAGS_JSON_TEMPLATE));
    created = true;
  }

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

  // Write updated tags.json
  await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2), 'utf-8');

  // Validate updated JSON against schema
  const validation = await validateTagsJson(tagsJsonPath);
  if (!validation.valid) {
    throw new Error(
      `Updated tags.json failed schema validation: ${validation.errors?.join(', ')}`
    );
  }

  // Regenerate TAGS.md from JSON
  const markdown = await generateTagsMd(tagsData);
  await writeFile(tagsMdPath, markdown, 'utf-8');

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
