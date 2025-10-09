import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';

interface DeleteTagOptions {
  tag: string;
  force?: boolean;
  dryRun?: boolean;
  cwd?: string;
}

interface DeleteTagResult {
  success: boolean;
  message?: string;
  warning?: string;
  error?: string;
}

export async function deleteTag(
  options: DeleteTagOptions
): Promise<DeleteTagResult> {
  const { tag, force = false, dryRun = false, cwd = process.cwd() } = options;
  const tagsPath = join(cwd, 'spec', 'TAGS.md');

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

  // Find the tag (looking for ### @tag pattern for H3 header format)
  const escapedTag = tag.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const tagHeaderPattern = new RegExp(`^### ${escapedTag}\\s*$`, 'm');
  const tagMatch = content.match(tagHeaderPattern);

  if (!tagMatch) {
    return {
      success: false,
      error: `Tag ${tag} not found in registry`,
    };
  }

  // Find which category the tag is in
  const tagIndex = tagMatch.index!;
  const beforeTag = content.substring(0, tagIndex);
  const categoryMatches = beforeTag.match(/^## (.+?)$/gm);
  const categoryName = categoryMatches
    ? categoryMatches[categoryMatches.length - 1].replace(/^## /, '')
    : 'Unknown';

  // Check if tag is used in any feature files (unless force or dry run)
  if (!force && !dryRun) {
    try {
      const files = await glob(['spec/features/**/*.feature'], {
        cwd,
        absolute: false,
      });
      const filesUsingTag: string[] = [];

      for (const file of files) {
        const fileContent = await readFile(join(cwd, file), 'utf-8');
        if (fileContent.includes(tag)) {
          filesUsingTag.push(file);
        }
      }

      if (filesUsingTag.length > 0) {
        return {
          success: false,
          error: `Tag ${tag} is used in ${filesUsingTag.length} feature file(s):\n  ${filesUsingTag.join('\n  ')}\n\nUse --force to delete anyway`,
        };
      }
    } catch {
      // If we can't check files, continue
    }
  }

  // If force and tag is in use, check for files and create warning
  let warning: string | undefined;
  if (force) {
    try {
      const files = await glob(['spec/features/**/*.feature'], {
        cwd,
        absolute: false,
      });
      const filesUsingTag: string[] = [];

      for (const file of files) {
        const fileContent = await readFile(join(cwd, file), 'utf-8');
        if (fileContent.includes(tag)) {
          filesUsingTag.push(file);
        }
      }

      if (filesUsingTag.length > 0) {
        warning = `Warning: Tag ${tag} is still used in ${filesUsingTag.length} file(s):\n  ${filesUsingTag.join('\n  ')}`;
      }
    } catch {
      // If we can't check files, continue
    }
  }

  // Dry run - just report what would happen
  if (dryRun) {
    return {
      success: true,
      message: `Would delete tag ${tag} from category "${categoryName}"`,
    };
  }

  // Find the tag block (### @tag + description lines until next ### or ##)
  const afterTag = content.substring(tagIndex);
  const nextHeaderMatch = afterTag.substring(tag.length + 4).match(/^##/m);
  const tagEnd = nextHeaderMatch
    ? tagIndex + tag.length + 4 + nextHeaderMatch.index!
    : content.length;

  // Remove the tag block
  const beforeRemoval = content.substring(0, tagIndex);
  const afterRemoval = content.substring(tagEnd);

  // Clean up extra blank lines
  let newContent = beforeRemoval + afterRemoval;

  // Remove excessive blank lines (more than 2 consecutive)
  newContent = newContent.replace(/\n{4,}/g, '\n\n\n');

  // Write the updated content
  await writeFile(tagsPath, newContent, 'utf-8');

  return {
    success: true,
    message: `Successfully deleted tag ${tag} from registry`,
    warning,
  };
}

export async function deleteTagCommand(
  tag: string,
  options: { force?: boolean; dryRun?: boolean }
): Promise<void> {
  try {
    const result = await deleteTag({
      tag,
      force: options.force,
      dryRun: options.dryRun,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (result.warning) {
      console.log(chalk.yellow(result.warning));
    }

    console.log(chalk.green(`✓ ${result.message}`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
