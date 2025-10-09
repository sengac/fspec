import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

interface ShowFoundationOptions {
  section?: string;
  format?: 'text' | 'markdown' | 'json';
  output?: string;
  listSections?: boolean;
  lineNumbers?: boolean;
  cwd?: string;
}

interface ShowFoundationResult {
  success: boolean;
  output?: string;
  format?: string;
  error?: string;
}

export async function showFoundation(
  options: ShowFoundationOptions
): Promise<ShowFoundationResult> {
  const {
    section,
    format = 'text',
    output,
    listSections = false,
    lineNumbers = false,
    cwd = process.cwd(),
  } = options;

  try {
    const foundationPath = join(cwd, 'spec/FOUNDATION.md');

    // Read FOUNDATION.md
    let content: string;
    try {
      content = await readFile(foundationPath, 'utf-8');
    } catch {
      return {
        success: false,
        error: 'FOUNDATION.md not found',
      };
    }

    // Parse sections
    const sections = parseSections(content);

    // List sections only
    if (listSections) {
      const sectionNames = Object.keys(sections).join('\n');

      if (output) {
        await writeFile(output, sectionNames, 'utf-8');
      }

      return {
        success: true,
        output: sectionNames,
        format: 'text',
      };
    }

    // Get specific section or all content
    let displayContent: string;

    if (section) {
      if (!sections[section]) {
        return {
          success: false,
          error: `Section '${section}' not found`,
        };
      }
      displayContent = sections[section];
    } else {
      displayContent = content;
    }

    // Format output
    let formattedOutput: string;

    if (format === 'json') {
      if (section) {
        formattedOutput = JSON.stringify(
          { [section]: displayContent },
          null,
          2
        );
      } else {
        formattedOutput = JSON.stringify(sections, null, 2);
      }
    } else if (format === 'markdown') {
      formattedOutput = displayContent;
    } else {
      // text format - remove markdown headers for cleaner display
      if (section) {
        // Remove the section header itself for text format
        const lines = displayContent.split('\n');
        const filteredLines = lines.filter(
          line => !line.trim().startsWith(`## ${section}`)
        );
        formattedOutput = filteredLines.join('\n').trim();
      } else {
        formattedOutput = displayContent;
      }
    }

    // Add line numbers if requested
    if (lineNumbers && format !== 'json') {
      const lines = formattedOutput.split('\n');
      formattedOutput = lines
        .map((line, index) => `${index + 1}: ${line}`)
        .join('\n');
    }

    // Write to file if output specified
    if (output) {
      await writeFile(output, formattedOutput, 'utf-8');
    }

    return {
      success: true,
      output: formattedOutput,
      format,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

function parseSections(content: string): Record<string, string> {
  const sections: Record<string, string> = {};
  const lines = content.split('\n');

  let currentSection: string | null = null;
  let currentContent: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Check if this is a section header (## but not ###)
    if (trimmed.startsWith('## ') && !trimmed.startsWith('### ')) {
      // Save previous section if exists
      if (currentSection !== null) {
        sections[currentSection] = currentContent.join('\n').trim();
      }

      // Start new section
      currentSection = trimmed.substring(3).trim();
      currentContent = [line]; // Include the header line
    } else if (currentSection !== null) {
      // Add line to current section
      currentContent.push(line);
    }
  }

  // Save last section
  if (currentSection !== null) {
    sections[currentSection] = currentContent.join('\n').trim();
  }

  return sections;
}

export async function showFoundationCommand(options: {
  section?: string;
  format?: string;
  output?: string;
  listSections?: boolean;
  lineNumbers?: boolean;
}): Promise<void> {
  try {
    const result = await showFoundation({
      section: options.section,
      format: (options.format as 'text' | 'markdown' | 'json') || 'text',
      output: options.output,
      listSections: options.listSections,
      lineNumbers: options.lineNumbers,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (!options.output) {
      console.log(result.output);
    } else {
      console.log(chalk.green('✓'), `Output written to ${options.output}`);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
