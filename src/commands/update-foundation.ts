import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

interface UpdateFoundationOptions {
  section: string;
  content: string;
  cwd?: string;
}

interface UpdateFoundationResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function updateFoundation(options: UpdateFoundationOptions): Promise<UpdateFoundationResult> {
  const { section, content, cwd = process.cwd() } = options;

  // Validate inputs
  if (!section || section.trim().length === 0) {
    return {
      success: false,
      error: 'Section name cannot be empty',
    };
  }

  if (content === undefined || content === null || content.trim().length === 0) {
    return {
      success: false,
      error: 'Section content cannot be empty',
    };
  }

  try {
    const foundationPath = join(cwd, 'spec/FOUNDATION.md');

    // Read or create FOUNDATION.md
    let fileContent = '';
    try {
      fileContent = await readFile(foundationPath, 'utf-8');
    } catch {
      // File doesn't exist, create new
      fileContent = '# Project Foundation\n\n';
    }

    // Split into lines for processing
    const lines = fileContent.split('\n');

    // Find the section
    let sectionStartIndex = -1;
    let sectionEndIndex = -1;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();

      // Found our section
      if (line === `## ${section}`) {
        sectionStartIndex = i;

        // Find where this section ends (next ## or end of file)
        for (let j = i + 1; j < lines.length; j++) {
          if (lines[j].trim().startsWith('## ')) {
            sectionEndIndex = j - 1;
            break;
          }
        }

        // If we didn't find another section, goes to end of file
        if (sectionEndIndex === -1) {
          sectionEndIndex = lines.length - 1;
        }

        break;
      }
    }

    // Prepare the new section content
    const contentLines = content.split('\n');

    if (sectionStartIndex === -1) {
      // Section doesn't exist, add it at the end
      if (lines[lines.length - 1].trim() !== '') {
        lines.push('');
      }
      lines.push(`## ${section}`, '', ...contentLines, '');
    } else {
      // Section exists, replace its content
      // Skip back over empty lines at the end of section
      while (sectionEndIndex > sectionStartIndex && lines[sectionEndIndex].trim() === '') {
        sectionEndIndex--;
      }

      // Replace the section content (keep the ## header, replace everything after)
      lines.splice(sectionStartIndex + 1, sectionEndIndex - sectionStartIndex, '', ...contentLines, '');
    }

    // Join back and write
    const newContent = lines.join('\n');
    await writeFile(foundationPath, newContent, 'utf-8');

    return {
      success: true,
      message: `Updated "${section}" section in FOUNDATION.md`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function updateFoundationCommand(section: string, content: string): Promise<void> {
  try {
    const result = await updateFoundation({
      section,
      content,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
