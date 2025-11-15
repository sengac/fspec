/**
 * AST Research Tool
 *
 * Code analysis using Abstract Syntax Tree parsing with deterministic tree-sitter queries.
 * Platform-agnostic TypeScript implementation.
 */

import type { ResearchTool } from './types';
import { QueryExecutor } from '../utils/query-executor';
import { loadLanguageParser } from '../utils/language-loader';
import * as fs from 'fs/promises';
import Parser from '@sengac/tree-sitter';

export const tool: ResearchTool = {
  name: 'ast',
  description:
    'AST code analysis tool using deterministic tree-sitter query operations. Supports 15 programming languages.',

  async execute(args: string[]): Promise<string> {
    // Helper function to parse argument (handles both --flag=value and --flag value)
    function parseArg(args: string[], flagName: string): string | undefined {
      const flagWithEquals = `${flagName}=`;

      // Check for --flag=value format
      const equalsArg = args.find(arg => arg.startsWith(flagWithEquals));
      if (equalsArg) {
        return equalsArg.substring(flagWithEquals.length);
      }

      // Check for --flag value format
      const index = args.indexOf(flagName);
      if (index >= 0 && index + 1 < args.length) {
        return args[index + 1];
      }

      return undefined;
    }

    // Parse arguments (handles both --flag=value and --flag value formats)
    const operation = parseArg(args, '--operation');
    const filePath = parseArg(args, '--file');
    const queryFile = parseArg(args, '--query-file');

    // Validate required flags
    if (!filePath) {
      throw new Error('--file is required');
    }

    if (!operation && !queryFile) {
      throw new Error('Either --operation or --query-file is required');
    }

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

    // Parse file with tree-sitter (lazy load parser)
    const parser = new Parser();
    const parserLanguage = await loadLanguageParser(language);
    parser.setLanguage(parserLanguage);

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
        'AST code analysis tool using deterministic tree-sitter query operations. Supports 15 languages: JavaScript, TypeScript, Python, Go, Rust, Kotlin, Dart, Swift, C#, C, C++, Java, PHP, Ruby, Bash',
      usage: 'fspec research --tool=ast [options]',
      whenToUse:
        'Use during Example Mapping to understand code structure, find patterns, or analyze existing implementations using deterministic tree-sitter queries.',
      prerequisites: [
        'Codebase must contain one of 15 supported languages: JavaScript, TypeScript, Python, Go, Rust, Kotlin, Dart, Swift, C#, C, C++, Java, PHP, Ruby, Bash',
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
        'Supports 15 languages: JavaScript, TypeScript, Python, Go, Rust, Kotlin, Dart, Swift, C#, C, C++, Java, PHP, Ruby, Bash',
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
  if (filePath.endsWith('.kt') || filePath.endsWith('.kts')) {
    return 'kotlin';
  }
  if (filePath.endsWith('.dart')) {
    return 'dart';
  }
  if (filePath.endsWith('.swift')) {
    return 'swift';
  }
  if (filePath.endsWith('.cs')) {
    return 'csharp';
  }
  if (filePath.endsWith('.c') || filePath.endsWith('.h')) {
    return 'c';
  }
  if (
    filePath.endsWith('.cpp') ||
    filePath.endsWith('.hpp') ||
    filePath.endsWith('.cc') ||
    filePath.endsWith('.cxx') ||
    filePath.endsWith('.hxx')
  ) {
    return 'cpp';
  }
  if (filePath.endsWith('.java')) {
    return 'java';
  }
  if (filePath.endsWith('.php')) {
    return 'php';
  }
  if (filePath.endsWith('.rb')) {
    return 'ruby';
  }
  if (filePath.endsWith('.sh') || filePath.endsWith('.bash')) {
    return 'bash';
  }
  throw new Error(`Cannot detect language from file extension: ${filePath}`);
}

export default tool;
