import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import type { Tags } from '../types/tags';
import { validateTagsJson } from '../validators/validate-json-schema';
import { generateTagsMd } from '../generators/tags-md';

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
  const tagsJsonPath = join(cwd, 'spec', 'tags.json');
  const tagsMdPath = join(cwd, 'spec', 'TAGS.md');

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
        message: `Would delete tag ${tag} from category "${currentCategory.name}"`,
      };
    }

    // Remove tag from category
    currentCategory.tags.splice(tagIndex, 1);

    // Validate updated JSON against schema
    const validation = validateTagsJson(tagsData);
    if (!validation.valid) {
      const errorMessages = validation.errors?.map(e => e.message).join(', ') || 'Unknown validation error';
      return {
        success: false,
        error: `Updated tags.json failed schema validation: ${errorMessages}`,
      };
    }

    // Write updated tags.json
    await writeFile(tagsJsonPath, JSON.stringify(tagsData, null, 2), 'utf-8');

    // Regenerate TAGS.md from JSON
    const markdown = await generateTagsMd(tagsData);
    await writeFile(tagsMdPath, markdown, 'utf-8');

    return {
      success: true,
      message: `Successfully deleted tag ${tag} from registry`,
      warning,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
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

    if (!options.dryRun) {
      console.log(chalk.gray('  Updated: spec/tags.json'));
      console.log(chalk.gray('  Regenerated: spec/TAGS.md'));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
