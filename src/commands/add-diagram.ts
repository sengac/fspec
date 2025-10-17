import { writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import { validateFoundationJson } from '../validators/validate-json-schema';
import { generateFoundationMd } from '../generators/foundation-md';
import { validateMermaidSyntax } from '../utils/mermaid-validation';
import { ensureFoundationFile } from '../utils/ensure-files';

interface AddDiagramOptions {
  section: string;
  title: string;
  code: string;
  description?: string;
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
  const { section, title, code, description, cwd = process.cwd() } = options;

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

  // Validate Mermaid syntax
  const validation = await validateMermaidSyntax(code);
  if (!validation.valid) {
    return {
      success: false,
      error: `Invalid Mermaid syntax: ${validation.error}`,
    };
  }

  try {
    const foundationJsonPath = join(cwd, 'spec/foundation.json');
    const foundationMdPath = join(cwd, 'spec/FOUNDATION.md');

    // Load or create foundation.json using ensureFoundationFile (generic schema v2.0.0)
    const foundationData: any = await ensureFoundationFile(cwd);

    // Ensure architectureDiagrams array exists
    if (!foundationData.architectureDiagrams) {
      foundationData.architectureDiagrams = [];
    }

    // Find existing diagram with same title or add new one
    const existingIndex = foundationData.architectureDiagrams.findIndex(
      (d: any) => d.title === title
    );

    // Create new diagram (generic schema doesn't have 'section' field)
    const newDiagram: any = {
      title,
      mermaidCode: code,
    };

    // Add optional description if provided
    if (description) {
      newDiagram.description = description;
    }

    if (existingIndex !== -1) {
      // Replace existing diagram
      foundationData.architectureDiagrams[existingIndex] = newDiagram;
    } else {
      // Add new diagram
      foundationData.architectureDiagrams.push(newDiagram);
    }

    // Write updated foundation.json
    await writeFile(
      foundationJsonPath,
      JSON.stringify(foundationData, null, 2),
      'utf-8'
    );

    // Validate updated JSON against schema
    const validation = validateFoundationJson(foundationData);
    if (!validation.valid) {
      const errorMessages = validation.errors?.map(e => e.message).join(', ');
      return {
        success: false,
        error: `Updated foundation.json failed schema validation: ${errorMessages}`,
      };
    }

    // Regenerate FOUNDATION.md from JSON
    const markdown = await generateFoundationMd(foundationData);
    await writeFile(foundationMdPath, markdown, 'utf-8');

    return {
      success: true,
      message:
        existingIndex !== -1
          ? `Updated diagram "${title}"`
          : `Added diagram "${title}"`,
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
    console.log(chalk.gray('  Updated: spec/foundation.json'));
    console.log(chalk.gray('  Regenerated: spec/FOUNDATION.md'));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerAddDiagramCommand(program: Command): void {
  program
    .command('add-diagram')
    .description('Add or update Mermaid diagram in FOUNDATION.md')
    .argument('<section>', 'Section name (e.g., "Architecture", "Data Flow")')
    .argument('<title>', 'Diagram title')
    .argument('<code>', 'Mermaid diagram code')
    .action(addDiagramCommand);
}
