import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

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

export async function updateTag(options: UpdateTagOptions): Promise<UpdateTagResult> {
  const { tag, category, description, cwd = process.cwd() } = options;
  const tagsPath = join(cwd, 'spec', 'TAGS.md');

  // Validate that at least one update is specified
  if (!category && !description) {
    return {
      success: false,
      error: 'No updates specified. Use --category and/or --description',
    };
  }

  // Read TAGS.md
  let content: string;
  try {
    content = await readFile(tagsPath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return {
        success: false,
        error: 'spec/TAGS.md not found',
      };
    }
    throw error;
  }

  // Check if tag exists (look for ### @tag pattern for H3 header format)
  const escapedTag = tag.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const tagHeaderPattern = new RegExp(`^### ${escapedTag}\\s*$`, 'm');
  const tagMatch = content.match(tagHeaderPattern);

  if (!tagMatch) {
    return {
      success: false,
      error: `Tag ${tag} not found in registry`,
    };
  }

  // Find which category the tag is currently in
  const tagIndex = tagMatch.index!;
  const beforeTag = content.substring(0, tagIndex);
  const currentCategoryMatch = beforeTag.match(/^## (.+?)$/gm);
  const currentCategory = currentCategoryMatch
    ? currentCategoryMatch[currentCategoryMatch.length - 1].replace(/^## /, '')
    : null;

  // If category is being changed, validate new category
  if (category && category !== currentCategory) {
    const categoryPattern = new RegExp(`^## ${category.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*$`, 'm');
    if (!categoryPattern.test(content)) {
      // Extract available categories
      const categoryMatches = content.match(/^## (.+)$/gm) || [];
      const availableCategories = categoryMatches.map(m => m.replace(/^## /, ''));
      return {
        success: false,
        error: `Invalid category: ${category}. Available categories: ${availableCategories.join(', ')}`,
      };
    }
  }

  // Find the tag's description (text between ### @tag and next ### or ##)
  const afterTag = content.substring(tagIndex);
  const nextHeaderMatch = afterTag.substring(tag.length + 4).match(/^##/m);
  const tagEnd = nextHeaderMatch
    ? tagIndex + tag.length + 4 + nextHeaderMatch.index!
    : content.length;

  const tagSection = content.substring(tagIndex, tagEnd);
  const descriptionMatch = tagSection.match(/^### .+?\n(.+?)(?=\n###|\n##|$)/s);
  const currentDescription = descriptionMatch ? descriptionMatch[1].trim() : '';

  // Determine the new description
  const newDescription = description || currentDescription;

  // If category is changing, remove tag from current location and insert in new category
  if (category && category !== currentCategory) {
    // Remove tag from current location
    const tagBlockStart = tagIndex;
    const tagBlockEnd = tagEnd;
    const beforeRemoval = content.substring(0, tagBlockStart);
    const afterRemoval = content.substring(tagBlockEnd);
    const contentWithoutTag = beforeRemoval + afterRemoval;

    // Insert tag in new category
    const newContent = insertTagInCategory(contentWithoutTag, category, tag, newDescription);
    await writeFile(tagsPath, newContent, 'utf-8');

    return {
      success: true,
      message: `Successfully updated ${tag}`,
    };
  } else {
    // Update description only (tag stays in same category)
    const tagHeader = `### ${tag}\n`;
    const newTagBlock = tagHeader + newDescription + '\n';

    const newContent =
      content.substring(0, tagIndex) + newTagBlock + content.substring(tagEnd);

    await writeFile(tagsPath, newContent, 'utf-8');

    return {
      success: true,
      message: `Successfully updated ${tag}`,
    };
  }
}

function insertTagInCategory(
  content: string,
  category: string,
  tag: string,
  description: string
): string {
  // Find the category section
  const escapedCategory = category.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const categoryPattern = new RegExp(`^## ${escapedCategory}\\s*$`, 'm');
  const categoryMatch = content.match(categoryPattern);
  if (!categoryMatch) {
    throw new Error(`Category not found: ${category}`);
  }

  const categoryIndex = categoryMatch.index!;

  // Find the end of the category (next ## or end of file)
  const afterCategory = content.substring(categoryIndex + 1);
  const nextCategoryMatch = afterCategory.match(/^## /m);
  const categoryEnd = nextCategoryMatch
    ? categoryIndex + 1 + nextCategoryMatch.index!
    : content.length;

  // Extract existing tags in this category (### @tag pattern)
  const categoryContent = content.substring(categoryIndex, categoryEnd);
  const tagPattern = /^### (@[a-z0-9-]+)\s*$/gm;

  const tags: Array<{ tag: string; index: number }> = [];
  let match;
  while ((match = tagPattern.exec(categoryContent)) !== null) {
    tags.push({
      tag: match[1],
      index: categoryIndex + match.index,
    });
  }

  // Find insertion point (alphabetical order)
  let insertIndex = categoryEnd;
  for (let i = 0; i < tags.length; i++) {
    if (tag.localeCompare(tags[i].tag) < 0) {
      insertIndex = tags[i].index;
      break;
    }
  }

  // Create new tag block
  const newTagBlock = `### ${tag}\n${description}\n\n`;

  // Insert the new tag block
  const newContent =
    content.substring(0, insertIndex) + newTagBlock + content.substring(insertIndex);

  return newContent;
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
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
