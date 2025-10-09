import { readFile, writeFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

interface AddDiagramOptions {
  section: string;
  title: string;
  code: string;
  cwd?: string;
}

interface AddDiagramResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function addDiagram(
  options: AddDiagramOptions
): Promise<AddDiagramResult> {
  const { section, title, code, cwd = process.cwd() } = options;

  // Validate inputs
  if (!section || section.trim().length === 0) {
    return {
      success: false,
      error: 'Section name cannot be empty',
    };
  }

  if (!title || title.trim().length === 0) {
    return {
      success: false,
      error: 'Diagram title cannot be empty',
    };
  }

  if (!code || code.trim().length === 0) {
    return {
      success: false,
      error: 'Diagram code cannot be empty',
    };
  }

  try {
    const foundationPath = join(cwd, 'spec/FOUNDATION.md');

    // Read or create FOUNDATION.md
    let content = '';
    try {
      content = await readFile(foundationPath, 'utf-8');
    } catch {
      // File doesn't exist, create new
      content = '# Project Foundation\n\n';
    }

    // Split into lines for processing
    const lines = content.split('\n');

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

    // Create the diagram markdown
    const diagramLines = [
      `### ${title}`,
      '',
      '```mermaid',
      ...code.split('\n'),
      '```',
      '',
    ];

    if (sectionStartIndex === -1) {
      // Section doesn't exist, add it at the end
      if (lines[lines.length - 1].trim() !== '') {
        lines.push('');
      }
      lines.push(`## ${section}`, '', ...diagramLines);
    } else {
      // Section exists, check if diagram with same title exists
      let diagramStartIndex = -1;
      let diagramEndIndex = -1;

      for (let i = sectionStartIndex + 1; i <= sectionEndIndex; i++) {
        const line = lines[i].trim();

        if (line === `### ${title}`) {
          diagramStartIndex = i;

          // Find the end of this diagram (next ### or next section)
          for (let j = i + 1; j <= sectionEndIndex; j++) {
            if (
              lines[j].trim().startsWith('### ') ||
              lines[j].trim().startsWith('## ')
            ) {
              diagramEndIndex = j - 1;
              break;
            }
          }

          // If we didn't find another subsection, goes to end of section
          if (diagramEndIndex === -1) {
            diagramEndIndex = sectionEndIndex;
          }

          break;
        }
      }

      if (diagramStartIndex !== -1) {
        // Replace existing diagram
        // Skip back over empty lines at the end
        while (
          diagramEndIndex > diagramStartIndex &&
          lines[diagramEndIndex].trim() === ''
        ) {
          diagramEndIndex--;
        }
        lines.splice(
          diagramStartIndex,
          diagramEndIndex - diagramStartIndex + 1,
          ...diagramLines
        );
      } else {
        // Add diagram at the end of the section
        // Find the last non-empty line in the section
        let insertIndex = sectionEndIndex;
        while (
          insertIndex > sectionStartIndex &&
          lines[insertIndex].trim() === ''
        ) {
          insertIndex--;
        }
        lines.splice(insertIndex + 1, 0, '', ...diagramLines);
      }
    }

    // Join back and write
    const newContent = lines.join('\n');
    await writeFile(foundationPath, newContent, 'utf-8');

    return {
      success: true,
      message: `Added diagram "${title}" to ${section} section`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function addDiagramCommand(
  section: string,
  title: string,
  code: string
): Promise<void> {
  try {
    const result = await addDiagram({
      section,
      title,
      code,
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
