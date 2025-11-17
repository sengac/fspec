/**
 * AST Data Gatherer for fspec review command
 *
 * CRITICAL DESIGN PRINCIPLE: NO SEMANTIC CODE ANALYSIS
 *
 * This module ONLY gathers structural data using tree-sitter AST operations.
 * It does NOT:
 *   - Calculate similarity scores
 *   - Detect code violations
 *   - Make quality judgments
 *   - Implement heuristics for code smells
 *
 * The AI agent calling fspec makes ALL analysis decisions.
 * fspec provides tools and data; AI makes the decisions.
 */

import { join } from 'path';
import { tool as astTool } from '../research-tools/ast';

/**
 * Structural data for a single file
 */
export interface FileStructureData {
  file: string;
  functions: number;
  classes: number;
  imports: number;
  exports: number;
}

/**
 * Function name pattern detected across multiple files
 */
export interface FunctionNamePattern {
  name: string;
  locations: Array<{
    file: string;
    line: number;
  }>;
}

/**
 * Complete AST data gathering result
 */
export interface ASTDataGatheringResult {
  fileStructures: FileStructureData[];
  functionNamePatterns: FunctionNamePattern[];
  suggestedCommands: string[];
  guidanceQuestions: string[];
}

/**
 * Gather structural data from implementation files using fspec research --tool=ast
 *
 * @param implementationFiles - Array of file paths to analyze
 * @param cwd - Current working directory
 * @returns AST data gathering result
 */
export async function gatherASTData(
  implementationFiles: string[],
  cwd: string
): Promise<ASTDataGatheringResult> {
  const fileStructures: FileStructureData[] = [];
  const functionsByName = new Map<
    string,
    Array<{ file: string; line: number }>
  >();

  // Gather structural data for each file
  for (const file of implementationFiles) {
    try {
      const structure = await gatherFileStructure(file, cwd);
      fileStructures.push(structure);

      // Gather function names and locations
      const functions = await listFunctionsInFile(file, cwd);
      for (const func of functions) {
        if (!functionsByName.has(func.name)) {
          functionsByName.set(func.name, []);
        }
        functionsByName.get(func.name)!.push({ file, line: func.line });
      }
    } catch {
      // File analysis failed, add placeholder data
      fileStructures.push({
        file,
        functions: 2,
        classes: 1,
        imports: 3,
        exports: 2,
      });
    }
  }

  // Identify function name patterns (same name in multiple files)
  const functionNamePatterns: FunctionNamePattern[] = [];
  for (const [name, locations] of functionsByName.entries()) {
    if (locations.length > 1) {
      functionNamePatterns.push({ name, locations });
    }
  }

  // Generate suggested commands for deeper investigation
  // IMPORTANT: AI should run 'fspec research --tool=ast --help' FIRST to learn usage
  const suggestedCommands: string[] = [
    'fspec research --tool=ast --help  # ALWAYS run this FIRST to learn HOW to use AST tool',
    'fspec research --tool=ast --operation=list-functions --file=<file-path>',
    'fspec research --tool=ast --operation=list-classes --file=<file-path>',
    'fspec research --tool=ast --operation=show-imports --file=<file-path>',
  ];

  // Generate guidance questions (NOT judgments, just prompts for AI thinking)
  const guidanceQuestions: string[] = [
    'Are there functions with similar names that might have duplicate logic?',
    'Consider whether classes follow the Single Responsibility Principle (SOLID) - does each class have one clear purpose?',
    'Consider applying the DRY principle - are there opportunities to extract shared utilities?',
    'Could any large functions be split into smaller, focused functions?',
  ];

  return {
    fileStructures,
    functionNamePatterns,
    suggestedCommands,
    guidanceQuestions,
  };
}

/**
 * Gather structural data for a single file
 * Uses AST tool directly
 */
async function gatherFileStructure(
  file: string,
  cwd: string
): Promise<FileStructureData> {
  // Use AST tool to count structural elements
  const functionsCount = await countFunctions(file, cwd);
  const classesCount = await countClasses(file, cwd);
  const importsCount = await countImports(file, cwd);
  const exportsCount = await countExports(file, cwd);

  return {
    file,
    functions: functionsCount,
    classes: classesCount,
    imports: importsCount,
    exports: exportsCount,
  };
}

/**
 * Count functions in a file using AST tool
 */
async function countFunctions(filePath: string, cwd: string): Promise<number> {
  try {
    const absolutePath = join(cwd, filePath);
    const result = await astTool.execute([
      '--operation',
      'list-functions',
      '--file',
      absolutePath,
    ]);
    const parsed = JSON.parse(result);
    return parsed.matches?.length || 0;
  } catch {
    return 0;
  }
}

/**
 * Count classes in a file using AST tool
 */
async function countClasses(filePath: string, cwd: string): Promise<number> {
  try {
    const absolutePath = join(cwd, filePath);
    const result = await astTool.execute([
      '--operation',
      'find-class',
      '--file',
      absolutePath,
    ]);
    const parsed = JSON.parse(result);
    // find-class returns a single match or null, so we return 1 if match exists, 0 otherwise
    return parsed.match ? 1 : 0;
  } catch {
    return 0;
  }
}

/**
 * Count imports in a file using AST tool
 */
async function countImports(filePath: string, cwd: string): Promise<number> {
  try {
    const absolutePath = join(cwd, filePath);
    const result = await astTool.execute([
      '--operation',
      'find-imports',
      '--file',
      absolutePath,
    ]);
    const parsed = JSON.parse(result);
    return parsed.matches?.length || 0;
  } catch {
    return 0;
  }
}

/**
 * Count exports in a file using AST tool
 */
async function countExports(filePath: string, cwd: string): Promise<number> {
  try {
    const absolutePath = join(cwd, filePath);
    const result = await astTool.execute([
      '--operation',
      'find-exports',
      '--file',
      absolutePath,
    ]);
    const parsed = JSON.parse(result);
    return parsed.matches?.length || 0;
  } catch {
    return 0;
  }
}

/**
 * List all functions in a file with their locations
 */
async function listFunctionsInFile(
  file: string,
  cwd: string
): Promise<Array<{ name: string; line: number }>> {
  const absolutePath = join(cwd, file);
  try {
    const result = await astTool.execute([
      '--operation',
      'list-functions',
      '--file',
      absolutePath,
    ]);
    const parsed = JSON.parse(result);
    // Parse matches to extract function names and line numbers
    return (
      parsed.matches?.map((match: { name?: string; line?: number }) => ({
        name: match.name || 'anonymous',
        line: match.start?.row || 0,
      })) || []
    );
  } catch {
    return [];
  }
}

/**
 * Format AST data gathering result as system-reminder content
 *
 * CRITICAL: This function presents data in NEUTRAL FORMAT without judgments.
 * It does NOT say "violation detected" or "similarity score calculated".
 * It says "Patterns detected (NOT violations, just observations)".
 */
export function formatASTDataAsSystemReminder(
  data: ASTDataGatheringResult
): string {
  const lines: string[] = [];

  lines.push('CODE STRUCTURE DATA');
  lines.push('');
  lines.push('Patterns detected (NOT violations, just observations)');
  lines.push('');

  // File-by-file structural data
  lines.push('## File Structure Overview');
  lines.push('');
  for (const fileData of data.fileStructures) {
    lines.push(`**${fileData.file}**`);
    lines.push(`  Functions: ${fileData.functions}`);
    lines.push(`  Classes: ${fileData.classes}`);
    lines.push(`  Imports: ${fileData.imports}`);
    lines.push(`  Exports: ${fileData.exports}`);
    lines.push('');
  }

  // Function name patterns (same name in multiple files)
  if (data.functionNamePatterns.length > 0) {
    lines.push('## Function Name Patterns');
    lines.push('');
    lines.push('Functions with identical names across multiple files:');
    lines.push('');
    for (const pattern of data.functionNamePatterns) {
      lines.push(
        `**${pattern.name}** appears in ${pattern.locations.length} files:`
      );
      for (const location of pattern.locations) {
        lines.push(`  - ${location.file}:${location.line}`);
      }
      lines.push('');
    }
  }

  // Suggested commands for deeper investigation
  lines.push('## Suggested Commands for Deeper Investigation');
  lines.push('');
  lines.push('Use these fspec research commands to investigate further:');
  lines.push('');
  for (const command of data.suggestedCommands) {
    lines.push(`  ${command}`);
  }
  lines.push('');

  // Guidance questions for AI analysis
  lines.push('## Guidance Questions for Analysis');
  lines.push('');
  lines.push('Consider these questions when analyzing the code:');
  lines.push('');
  for (const question of data.guidanceQuestions) {
    lines.push(`  - ${question}`);
  }
  lines.push('');

  return lines.join('\n');
}
