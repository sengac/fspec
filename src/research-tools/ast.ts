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

  help(): string {
    return `AST RESEARCH TOOL

Research code structure using AST parsing during Example Mapping.

USAGE
  ast --query <query> [options]
  ast --file <path> [options]

OPTIONS
  --query <query>     Natural language query for pattern detection (required if no --file)
  --file <path>       Specific file to analyze (required if no --query)
  --format <type>     Output format: json, markdown, text (default: json)
  --language <lang>   Language filter: typescript, python, go, rust, etc.
  --help              Show this help message

QUERY EXAMPLES
  ast --query "find all async functions"
  ast --query "functions with more than 5 parameters"
  ast --query "classes implementing interface UserRepository"

FILE EXAMPLES
  ast --file "src/broken.ts"
  ast --file "src/auth/login.ts"

FEATURES
  - AST parsing using tree-sitter (supports 40+ languages)
  - Pattern detection across TypeScript, JavaScript, Python, Go, Rust, Java, C++
  - Error-tolerant parsing (analyzes incomplete or broken code)

EXIT CODES
  0  Success
  1  Missing required flag (--query or --file)
  2  File not found or parsing error
  3  Invalid query or unsupported language`;
  },
};

export default tool;
