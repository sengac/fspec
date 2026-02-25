/**
 * Source Code Analysis Test Fixtures
 *
 * TUI-076: Utilities for verifying type definitions and imports in source files.
 *
 * These fixtures provide:
 * - File discovery (find TypeScript files recursively)
 * - Pattern matching (grep for patterns across files)
 * - Type definition verification
 * - Import statement verification
 *
 * SOLID: Single Responsibility - Only handles source code analysis
 * DRY: Reusable across type consolidation and architecture tests
 * COMPOSABLE: Pure functions, no side effects
 */

import { readdirSync, readFileSync, existsSync } from 'fs';
import { join, relative } from 'path';

// ============================================================================
// FILE DISCOVERY
// ============================================================================

/**
 * Recursively find all TypeScript files in a directory.
 *
 * @param dir - Directory to search
 * @param extensions - File extensions to match (default: .ts, .tsx)
 * @returns Array of absolute file paths
 *
 * @example
 * ```typescript
 * const files = findTypeScriptFiles('/path/to/src/tui');
 * // ['/path/to/src/tui/types/provider.ts', '/path/to/src/tui/components/AgentView.tsx', ...]
 * ```
 */
export function findTypeScriptFiles(
  dir: string,
  extensions: string[] = ['.ts', '.tsx']
): string[] {
  const results: string[] = [];

  if (!existsSync(dir)) {
    return results;
  }

  const entries = readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);

    if (entry.isDirectory()) {
      // Skip node_modules and hidden directories
      if (!entry.name.startsWith('.') && entry.name !== 'node_modules') {
        results.push(...findTypeScriptFiles(fullPath, extensions));
      }
    } else if (entry.isFile()) {
      const hasMatchingExtension = extensions.some(ext =>
        entry.name.endsWith(ext)
      );
      if (hasMatchingExtension) {
        results.push(fullPath);
      }
    }
  }

  return results;
}

// ============================================================================
// PATTERN MATCHING
// ============================================================================

/**
 * Result of a pattern search across files.
 */
export interface PatternMatch {
  /** Absolute path to the file */
  absolutePath: string;
  /** Path relative to search directory */
  relativePath: string;
  /** Line number (1-indexed) */
  line: number;
  /** The matched text */
  match: string;
}

/**
 * Search for a pattern across TypeScript files, returning matches with context.
 *
 * @param dir - Directory to search
 * @param pattern - RegExp pattern to search for
 * @returns Array of matches with file and line information
 *
 * @example
 * ```typescript
 * const matches = grepTypeScriptFiles(TUI_DIR, /interface\s+ModelSelection/);
 * expect(matches).toHaveLength(1);
 * expect(matches[0].relativePath).toBe('types/provider.ts');
 * ```
 */
export function grepTypeScriptFiles(
  dir: string,
  pattern: RegExp
): PatternMatch[] {
  const files = findTypeScriptFiles(dir);
  const matches: PatternMatch[] = [];

  for (const file of files) {
    const content = readFileSync(file, 'utf-8');
    const lines = content.split('\n');

    for (let i = 0; i < lines.length; i++) {
      const lineContent = lines[i];
      const match = lineContent.match(pattern);

      if (match) {
        matches.push({
          absolutePath: file,
          relativePath: relative(dir, file),
          line: i + 1,
          match: match[0],
        });
      }
    }
  }

  return matches;
}

/**
 * Find files containing a pattern (simpler version of grep).
 *
 * @param dir - Directory to search
 * @param pattern - RegExp pattern to search for
 * @returns Array of relative file paths that contain the pattern
 *
 * @example
 * ```typescript
 * const files = findFilesWithPattern(TUI_DIR, /export\s+interface\s+ModelSelection/);
 * expect(files).toEqual(['types/provider.ts']);
 * ```
 */
export function findFilesWithPattern(dir: string, pattern: RegExp): string[] {
  const files = findTypeScriptFiles(dir);
  const matches: string[] = [];

  for (const file of files) {
    const content = readFileSync(file, 'utf-8');
    if (pattern.test(content)) {
      matches.push(relative(dir, file));
    }
  }

  return matches;
}

// ============================================================================
// TYPE DEFINITION VERIFICATION
// ============================================================================

/**
 * Verifies that a type/interface is exported exactly once from a specific file.
 *
 * @param dir - Directory to search
 * @param typeName - Name of the type/interface
 * @param expectedFile - Expected file (relative path)
 * @param isInterface - Whether it's an interface (true) or type alias (false)
 * @returns Verification result with details
 *
 * @example
 * ```typescript
 * const result = verifyTypeDefinition(TUI_DIR, 'ModelSelection', 'types/provider.ts', true);
 * expect(result.isValid).toBe(true);
 * expect(result.duplicates).toHaveLength(0);
 * ```
 */
export function verifyTypeDefinition(
  dir: string,
  typeName: string,
  expectedFile: string,
  isInterface: boolean
): {
  isValid: boolean;
  foundIn: string[];
  duplicates: string[];
  expectedFileHasDefinition: boolean;
} {
  const keyword = isInterface ? 'interface' : 'type';
  const pattern = new RegExp(`export\\s+${keyword}\\s+${typeName}\\s*[={]`);

  const filesWithDefinition = findFilesWithPattern(dir, pattern);
  const duplicates = filesWithDefinition.filter(f => f !== expectedFile);
  const expectedFileHasDefinition = filesWithDefinition.includes(expectedFile);

  return {
    isValid: expectedFileHasDefinition && duplicates.length === 0,
    foundIn: filesWithDefinition,
    duplicates,
    expectedFileHasDefinition,
  };
}

// ============================================================================
// IMPORT VERIFICATION
// ============================================================================

/**
 * Extracts imported names from a specific import path.
 *
 * @param fileContent - Content of the file to analyze
 * @param importPath - The import path to look for (e.g., '../types/provider')
 * @returns Array of imported names, or null if import not found
 *
 * @example
 * ```typescript
 * const content = readFileSync('AgentView.tsx', 'utf-8');
 * const imports = extractImports(content, '../types/provider');
 * expect(imports).toContain('ModelSelection');
 * expect(imports).toContain('ProviderSection');
 * ```
 */
export function extractImports(
  fileContent: string,
  importPath: string
): string[] | null {
  // Escape special regex characters in the import path
  const escapedPath = importPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Match both: import { X } from 'path' and import type { X } from 'path'
  const importRegex = new RegExp(
    `import\\s+(?:type\\s+)?{([^}]+)}\\s+from\\s+['"]${escapedPath}['"]`
  );

  const match = fileContent.match(importRegex);
  if (!match) {
    return null;
  }

  // Parse the imported names, handling multiline imports
  const importClause = match[1];
  const names = importClause
    .split(',')
    .map(name => name.trim())
    .filter(name => name.length > 0);

  return names;
}

/**
 * Verifies that a file imports specific types from a given path.
 *
 * @param filePath - Path to the file to check
 * @param importPath - The import path to look for
 * @param expectedTypes - Types that should be imported
 * @returns Verification result
 *
 * @example
 * ```typescript
 * const result = verifyImports(
 *   '/path/to/AgentView.tsx',
 *   '../types/provider',
 *   ['ModelSelection', 'ModelSelectorItem', 'ProviderSection']
 * );
 * expect(result.allFound).toBe(true);
 * ```
 */
export function verifyImports(
  filePath: string,
  importPath: string,
  expectedTypes: string[]
): {
  allFound: boolean;
  found: string[];
  missing: string[];
  importExists: boolean;
} {
  if (!existsSync(filePath)) {
    return {
      allFound: false,
      found: [],
      missing: expectedTypes,
      importExists: false,
    };
  }

  const content = readFileSync(filePath, 'utf-8');
  const imports = extractImports(content, importPath);

  if (imports === null) {
    return {
      allFound: false,
      found: [],
      missing: expectedTypes,
      importExists: false,
    };
  }

  const found = expectedTypes.filter(type => imports.includes(type));
  const missing = expectedTypes.filter(type => !imports.includes(type));

  return {
    allFound: missing.length === 0,
    found,
    missing,
    importExists: true,
  };
}
