/**
 * AST Research Tool
 *
 * Code analysis using Abstract Syntax Tree parsing.
 * Platform-agnostic TypeScript implementation.
 */

import type { ResearchTool } from './types';
import { parseFile } from '../utils/ast-parser';
import { glob } from 'tinyglobby';

export const tool: ResearchTool = {
  name: 'ast',
  description:
    'AST code analysis tool for pattern detection and deep code analysis',

  async execute(args: string[]): Promise<string> {
    // Parse arguments
    const queryIndex = args.indexOf('--query');
    const fileIndex = args.indexOf('--file');

    if (queryIndex === -1 && fileIndex === -1) {
      throw new Error('At least one of --query or --file is required');
    }

    const query = queryIndex >= 0 ? args[queryIndex + 1] : undefined;
    const filePath = fileIndex >= 0 ? args[fileIndex + 1] : undefined;

    // If file path provided, parse that specific file
    if (filePath) {
      const result = await parseFile(filePath, query);
      return JSON.stringify(result, null, 2);
    }

    // If query provided without file, search all TypeScript/JavaScript files
    if (query) {
      const files = await glob('**/*.{ts,js,tsx,jsx}', {
        ignore: [
          '**/node_modules/**',
          '**/dist/**',
          '**/__tests__/**',
          '**/build/**',
          '**/.next/**',
        ],
      });

      const allMatches = [];
      for (const file of files) {
        // Process ALL files
        try {
          const result = await parseFile(file, query);
          if (result.matches && result.matches.length > 0) {
            allMatches.push(...result.matches);
          }
        } catch {
          // Skip files that can't be parsed
          continue;
        }
      }

      return JSON.stringify({ query, matches: allMatches }, null, 2);
    }

    throw new Error('No valid query or file provided');
  },

  getHelpConfig() {
    return {
      name: 'ast',
      description:
        'AST code analysis tool for pattern detection and deep code analysis',
      usage: 'fspec research --tool=ast [options]',
      whenToUse:
        'Use during Example Mapping to understand code structure, find patterns, or analyze existing implementations before writing specifications.',
      prerequisites: [
        'Codebase must contain TypeScript, JavaScript, Python, Go, Rust, or Java files',
        'Tree-sitter parsers are bundled (no additional setup required)',
      ],
      options: [
        {
          flag: '--query <query>',
          description:
            'Natural language query for pattern detection (required if no --file)',
        },
        {
          flag: '--file <path>',
          description: 'Specific file to analyze (required if no --query)',
        },
        {
          flag: '--format <type>',
          description: 'Output format',
          defaultValue: 'json',
        },
        {
          flag: '--language <lang>',
          description: 'Language filter: typescript, python, go, rust, etc.',
        },
      ],
      examples: [
        {
          command: '--query "find all async functions"',
          description: 'Find all async function definitions',
        },
        {
          command: '--query "functions with more than 5 parameters"',
          description: 'Detect functions with high parameter count',
        },
        {
          command: '--file "src/auth/login.ts"',
          description: 'Analyze specific file structure',
        },
      ],
      features: [
        'AST parsing using tree-sitter (supports 40+ languages)',
        'Pattern detection across TypeScript, JavaScript, Python, Go, Rust, Java, C++',
        'Error-tolerant parsing (analyzes incomplete or broken code)',
      ],
      commonErrors: [
        {
          error: 'At least one of --query or --file is required',
          fix: 'Provide either --query "your query" or --file "path/to/file"',
        },
        {
          error: 'File not found or parsing error',
          fix: 'Check file path exists and is a valid source file',
        },
      ],
      exitCodes: [
        { code: 0, description: 'Success' },
        { code: 1, description: 'Missing required flag (--query or --file)' },
        { code: 2, description: 'File not found or parsing error' },
        { code: 3, description: 'Invalid query or unsupported language' },
      ],
    };
  },
};

export default tool;
