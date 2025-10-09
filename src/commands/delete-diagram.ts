import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Foundation } from '../types/foundation';
import { generateFoundationMd } from '../generators/foundation-md';

interface DeleteDiagramOptions {
  section: string;
  title: string;
  cwd?: string;
}

interface DeleteDiagramResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function deleteDiagram(
  options: DeleteDiagramOptions
): Promise<DeleteDiagramResult> {
  const { section, title, cwd = process.cwd() } = options;

  const foundationJsonPath = join(cwd, 'spec/foundation.json');

  // Check if foundation.json exists
  if (!existsSync(foundationJsonPath)) {
    return {
      success: false,
      error: 'foundation.json not found: spec/foundation.json',
    };
  }

  try {
    // Read foundation.json
    const content = await readFile(foundationJsonPath, 'utf-8');
    const foundationData: Foundation = JSON.parse(content);

    // Find diagram index
    const diagramIndex = foundationData.architectureDiagrams.findIndex(
      d => d.section === section && d.title === title
    );

    if (diagramIndex === -1) {
      return {
        success: false,
        error: `Diagram '${title}' not found in section '${section}'`,
      };
    }

    // Remove diagram from array
    foundationData.architectureDiagrams.splice(diagramIndex, 1);

    // Write updated foundation.json
    await writeFile(
      foundationJsonPath,
      JSON.stringify(foundationData, null, 2),
      'utf-8'
    );

    // Regenerate FOUNDATION.md
    const foundationMdPath = join(cwd, 'spec/FOUNDATION.md');
    const markdown = await generateFoundationMd(foundationData);
    await writeFile(foundationMdPath, markdown, 'utf-8');

    return {
      success: true,
      message: `Deleted diagram '${title}' from section '${section}'`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function deleteDiagramCommand(
  section: string,
  title: string
): Promise<void> {
  try {
    const result = await deleteDiagram({ section, title });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);
    console.log(chalk.gray('  Updated: spec/foundation.json'));
    console.log(chalk.gray('  Regenerated: spec/FOUNDATION.md'));

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
