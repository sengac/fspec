/**
 * AST Research Tool
 *
 * Code analysis using Abstract Syntax Tree parsing with deterministic tree-sitter queries.
 * Platform-agnostic TypeScript implementation.
 */

import type { ResearchTool } from './types';
import { QueryExecutor } from '../utils/query-executor';
import * as fs from 'fs/promises';
import Parser from 'tree-sitter';
// @ts-expect-error - tree-sitter-javascript doesn't have type definitions
import JavaScript from 'tree-sitter-javascript';
// @ts-expect-error - tree-sitter-typescript doesn't have type definitions
import TypeScript from 'tree-sitter-typescript';

export const tool: ResearchTool = {
  name: 'ast',
  description:
    'AST code analysis tool using deterministic tree-sitter query operations',

  async execute(args: string[]): Promise<string> {
    // Parse arguments
    const operationIndex = args.indexOf('--operation');
    const fileIndex = args.indexOf('--file');
    const queryFileIndex = args.indexOf('--query-file');

    // Validate required flags
    if (fileIndex === -1) {
      throw new Error('--file is required');
    }

    if (operationIndex === -1 && queryFileIndex === -1) {
      throw new Error('Either --operation or --query-file is required');
    }

    const operation =
      operationIndex >= 0 ? args[operationIndex + 1] : undefined;
    const filePath = args[fileIndex + 1];
    const queryFile =
      queryFileIndex >= 0 ? args[queryFileIndex + 1] : undefined;

    // Extract parameters
    const parameters: Record<string, string> = {};
    for (let i = 0; i < args.length; i++) {
      if (
        args[i].startsWith('--') &&
        args[i + 1] &&
        !args[i + 1].startsWith('--')
      ) {
        const key = args[i].replace('--', '');
        if (key !== 'operation' && key !== 'file' && key !== 'query-file') {
          parameters[key] = args[i + 1];
        }
      }
    }

    // Detect language from file extension
    const language = detectLanguage(filePath);

    // Create query executor
    const executor = new QueryExecutor({
      language,
      operation,
      queryFile,
      parameters,
    });

    // Parse file with tree-sitter
    const parser = new Parser();
    let parserLanguage;
    if (language === 'javascript') {
      parserLanguage = JavaScript;
      parser.setLanguage(JavaScript);
    } else if (language === 'typescript') {
      parserLanguage = TypeScript.typescript;
      parser.setLanguage(TypeScript.typescript);
    } else {
      throw new Error(`Unsupported language: ${language}`);
    }

    const fileContent = await fs.readFile(filePath, 'utf-8');
    const tree = parser.parse(fileContent);

    // Execute query
    const matches = await executor.execute(tree, parserLanguage);

    // Format result based on operation
    if (operation === 'find-class' || operation === 'find-exports') {
      // Single match operations
      const match = matches[0];
      return JSON.stringify({ match: match || null }, null, 2);
    }

    // Multiple match operations
    return JSON.stringify({ matches }, null, 2);
  },

  getHelpConfig() {
    return {
      name: 'ast',
      description:
        'AST code analysis tool using deterministic tree-sitter query operations',
      usage: 'fspec research --tool=ast [options]',
      whenToUse:
        'Use during Example Mapping to understand code structure, find patterns, or analyze existing implementations using deterministic tree-sitter queries.',
      prerequisites: [
        'Codebase must contain TypeScript, JavaScript, Python, Go, Rust, or Java files',
        'Tree-sitter parsers are bundled (no additional setup required)',
      ],
      options: [
        {
          flag: '--operation <operation>',
          description:
            'Predefined query operation: list-functions, find-class, find-async-functions, etc. (required if no --query-file)',
        },
        {
          flag: '--query-file <path>',
          description:
            'Path to custom tree-sitter query file (.scm) (required if no --operation)',
        },
        {
          flag: '--file <path>',
          description: 'Specific file to analyze (required)',
        },
        {
          flag: '--name <name>',
          description: 'Filter by name (for find-class, find-function)',
        },
        {
          flag: '--pattern <regex>',
          description: 'Filter by regex pattern (for find-identifiers)',
        },
        {
          flag: '--min-params <count>',
          description: 'Minimum parameter count (for find-functions)',
        },
        {
          flag: '--export-type <type>',
          description: 'Export type filter: default, named (for find-exports)',
        },
      ],
      examples: [
        {
          command: '--operation=list-functions --file=src/auth.ts',
          description:
            'List all function declarations, expressions, and arrow functions',
        },
        {
          command:
            '--operation=find-class --name=AuthController --file=src/auth.ts',
          description: 'Find specific class by name',
        },
        {
          command:
            '--operation=find-functions --min-params=5 --file=src/api.ts',
          description: 'Find functions with 5 or more parameters',
        },
        {
          command:
            '--operation=find-identifiers --pattern="^[A-Z][A-Z_]+" --file=src/constants.ts',
          description: 'Find CONSTANT_CASE identifiers',
        },
        {
          command: '--query-file=queries/custom.scm --file=src/utils.ts',
          description: 'Execute custom tree-sitter query from file',
        },
      ],
      features: [
        'Deterministic tree-sitter query language (S-expressions)',
        'Predefined operations for common patterns',
        'Custom query support via .scm files',
        'Parametric predicates for filtering (name, pattern, min-params)',
        'Supports TypeScript, JavaScript, Python, Go, Rust',
      ],
      commonErrors: [
        {
          error: '--file is required',
          fix: 'Provide --file path/to/file.ts',
        },
        {
          error: 'Either --operation or --query-file is required',
          fix: 'Provide either --operation=list-functions or --query-file=custom.scm',
        },
        {
          error: 'Unknown operation',
          fix: 'Use valid operation: list-functions, find-class, find-async-functions, etc.',
        },
      ],
      exitCodes: [
        { code: 0, description: 'Success' },
        {
          code: 1,
          description:
            'Missing required flag (--file, --operation, or --query-file)',
        },
        { code: 2, description: 'File not found or parsing error' },
        { code: 3, description: 'Invalid operation or unsupported language' },
      ],
    };
  },
};

function detectLanguage(filePath: string): string {
  if (filePath.endsWith('.ts') || filePath.endsWith('.tsx')) {
    return 'typescript';
  }
  if (filePath.endsWith('.js') || filePath.endsWith('.jsx')) {
    return 'javascript';
  }
  if (filePath.endsWith('.py')) {
    return 'python';
  }
  if (filePath.endsWith('.go')) {
    return 'go';
  }
  if (filePath.endsWith('.rs')) {
    return 'rust';
  }
  throw new Error(`Cannot detect language from file extension: ${filePath}`);
}

export default tool;
