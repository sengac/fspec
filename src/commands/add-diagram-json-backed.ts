import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { Foundation } from '../types/foundation';
import { validateFoundationJson as validateFoundationJsonFile } from '../validators/json-schema';
import { generateFoundationMd } from '../generators/foundation-md';

interface Diagram {
  title: string;
  mermaidCode: string;
  description?: string;
}

interface AddDiagramResult {
  success: boolean;
  warning?: string;
}

export async function addDiagramJsonBacked(options: {
  title: string;
  mermaidCode?: string;
  file?: string;
  description?: string;
  section?: string;
  cwd?: string;
  forceRegenerationFailure?: boolean;
}): Promise<AddDiagramResult> {
  const cwd = options.cwd || process.cwd();
  const foundationFile = join(cwd, 'spec', 'foundation.json');
  const foundationMdFile = join(cwd, 'spec', 'FOUNDATION.md');

  try {
    // Read foundation.json
    const content = await readFile(foundationFile, 'utf-8');
    const originalContent = content;
    const data: Foundation = JSON.parse(content);

    // Validate section (if provided)
    if (options.section && options.section !== 'Architecture Diagrams') {
      throw new Error(`Section '${options.section}' not found`);
    }

    // Get Mermaid code from file or parameter
    let mermaidCode = options.mermaidCode || '';
    if (options.file) {
      mermaidCode = await readFile(options.file, 'utf-8');
    }

    // Validate Mermaid syntax (warning only)
    let warning: string | undefined;
    try {
      await validateMermaidSyntax(mermaidCode);
    } catch {
      warning = '⚠ Mermaid syntax may be invalid';
    }

    // Check if diagram with same title exists (update instead of duplicate)
    const existingIndex = data.architectureDiagrams.findIndex(
      d => d.title === options.title
    );

    const newDiagram: Diagram = {
      title: options.title,
      mermaidCode,
    };

    if (options.description) {
      newDiagram.description = options.description;
    }

    if (existingIndex >= 0) {
      // Update existing diagram
      data.architectureDiagrams[existingIndex] = newDiagram;
    } else {
      // Add new diagram
      data.architectureDiagrams.push(newDiagram);
    }

    // Write updated foundation.json
    await writeFile(foundationFile, JSON.stringify(data, null, 2));

    // Validate updated foundation.json against schema
    const validation = await validateFoundationJsonFile(foundationFile);
    if (!validation.valid) {
      // Rollback on validation failure
      await writeFile(foundationFile, originalContent);
      throw new Error(`foundation.json validation failed: ${validation.errors?.join(', ')}`);
    }

    // Regenerate FOUNDATION.md
    try {
      if (options.forceRegenerationFailure) {
        throw new Error('Forced regeneration failure for testing');
      }

      const markdown = await generateFoundationMd(data);
      await writeFile(foundationMdFile, markdown, 'utf-8');
    } catch (error: unknown) {
      // Rollback foundation.json on markdown generation failure
      await writeFile(foundationFile, originalContent);
      if (error instanceof Error) {
        throw new Error(`Failed to regenerate FOUNDATION.md - changes rolled back: ${error.message}`);
      }
      throw new Error('Failed to regenerate FOUNDATION.md - changes rolled back');
    }

    return {
      success: true,
      warning,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to add diagram: ${error.message}`);
    }
    throw error;
  }
}

async function validateMermaidSyntax(mermaidCode: string): Promise<void> {
  // Basic validation: check if it looks like valid Mermaid
  // More sophisticated validation would use mermaid.parse() with jsdom
  const validPrefixes = ['graph', 'flowchart', 'sequenceDiagram', 'classDiagram', 'stateDiagram', 'erDiagram', 'journey', 'gantt', 'pie', 'gitGraph'];

  const trimmed = mermaidCode.trim();
  const startsWithValid = validPrefixes.some(prefix =>
    trimmed.startsWith(prefix) || trimmed.startsWith(`${prefix}-`)
  );

  if (!startsWithValid) {
    throw new Error('Invalid Mermaid syntax');
  }
}
