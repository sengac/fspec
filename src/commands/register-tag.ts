import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

interface RegisterTagOptions {
  cwd?: string;
}

interface RegisterTagResult {
  success: boolean;
  message: string;
  created?: boolean;
  converted?: boolean;
}

const TAGS_MD_TEMPLATE = `# Tag Registry

## Phase Tags
| Tag | Description |
|-----|-------------|

## Component Tags
| Tag | Description |
|-----|-------------|

## Feature Group Tags
| Tag | Description |
|-----|-------------|

## Technical Tags
| Tag | Description |
|-----|-------------|

## Platform Tags
| Tag | Description |
|-----|-------------|

## Priority Tags
| Tag | Description |
|-----|-------------|

## Status Tags
| Tag | Description |
|-----|-------------|

## Testing Tags
| Tag | Description |
|-----|-------------|

## CAGE Integration Tags
| Tag | Description |
|-----|-------------|
`;

export async function registerTag(
  tag: string,
  category: string,
  description: string,
  options: RegisterTagOptions = {}
): Promise<RegisterTagResult> {
  const cwd = options.cwd || process.cwd();
  const tagsPath = join(cwd, 'spec', 'TAGS.md');

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

  // Read or create TAGS.md
  let content: string;
  let created = false;

  try {
    content = await readFile(tagsPath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      // Create spec directory and TAGS.md
      await mkdir(join(cwd, 'spec'), { recursive: true });
      content = TAGS_MD_TEMPLATE;
      created = true;
    } else {
      throw error;
    }
  }

  // Check for duplicate
  const escapedTag = normalizedTag.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const tagPattern = new RegExp(`\\|\\s*\`${escapedTag}\`\\s*\\|`, 'g');
  if (tagPattern.test(content)) {
    throw new Error(`Tag ${normalizedTag} is already registered in TAGS.md`);
  }

  // Find and validate category
  const categoryPattern = new RegExp(`^## ${category}\\s*$`, 'mi');
  if (!categoryPattern.test(content)) {
    // Extract available categories
    const categoryMatches = content.match(/^## (.+)$/gm) || [];
    const availableCategories = categoryMatches.map(m => m.replace(/^## /, ''));
    throw new Error(
      `Invalid category: "${category}". Available categories: ${availableCategories.join(', ')}`
    );
  }

  // Insert tag in alphabetical order within the category
  content = insertTagInCategory(content, category, normalizedTag, description);

  // Write updated content
  await writeFile(tagsPath, content, 'utf-8');

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

function insertTagInCategory(
  content: string,
  category: string,
  tag: string,
  description: string
): string {
  // Find the category section
  const categoryPattern = new RegExp(`^## ${category}\\s*$`, 'mi');
  const categoryMatch = content.match(categoryPattern);
  if (!categoryMatch) {
    throw new Error(`Category not found: ${category}`);
  }

  const categoryIndex = categoryMatch.index!;

  // Find the table header (next occurrence of |-----|)
  const tableStart = content.indexOf('|-----|', categoryIndex);
  if (tableStart === -1) {
    throw new Error(`Table not found in category: ${category}`);
  }

  // Find the end of the category (next ## or end of file)
  const nextCategoryMatch = content.substring(categoryIndex + 1).match(/^## /m);
  const categoryEnd = nextCategoryMatch
    ? categoryIndex + 1 + nextCategoryMatch.index!
    : content.length;

  // Extract existing tags in this category
  const categoryContent = content.substring(tableStart, categoryEnd);
  const tagLines: Array<{ tag: string; line: string }> = [];
  const tagPattern = /\|\s*`(@[a-z0-9-]+)`\s*\|(.+?)\|/g;

  let match;
  while ((match = tagPattern.exec(categoryContent)) !== null) {
    tagLines.push({
      tag: match[1],
      line: match[0],
    });
  }

  // Create new tag line
  const newTagLine = `| \`${tag}\` | ${description} |`;

  // Find insertion point (alphabetical order)
  let insertAfterLine = '|-----|-------------|';
  for (const tagLine of tagLines) {
    if (tag.localeCompare(tagLine.tag) > 0) {
      insertAfterLine = tagLine.line;
    } else {
      break;
    }
  }

  // Insert the new tag line
  const insertIndex = content.indexOf(insertAfterLine, tableStart) + insertAfterLine.length;
  const newContent =
    content.substring(0, insertIndex) +
    '\n' +
    newTagLine +
    content.substring(insertIndex);

  return newContent;
}

export async function registerTagCommand(
  tag: string,
  category: string,
  description: string
): Promise<void> {
  try {
    const result = await registerTag(tag, category, description);

    if (result.created) {
      console.log(chalk.yellow('Created new TAGS.md file'));
    }

    if (result.converted) {
      console.log(chalk.yellow(`Note: Tag converted to lowercase: ${tag} → ${tag.toLowerCase()}`));
    }

    console.log(chalk.green(`✓ ${result.message}`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
