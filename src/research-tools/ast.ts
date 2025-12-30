/**
 * AST Research Tool
 *
 * Code analysis using AST-based pattern matching via codelet's native ast-grep implementation.
 * Uses ast-grep pattern syntax for structural code search and refactoring.
 */

import type { ResearchTool } from './types';
import { astGrepSearch, astGrepRefactor } from '@sengac/codelet-napi';

export const tool: ResearchTool = {
  name: 'ast',
  description:
    'AST-based code search and refactoring using ast-grep pattern matching. Supports 23 programming languages.',

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

    // Check for --refactor flag
    const isRefactor = args.includes('--refactor');

    // Parse arguments
    const pattern = parseArg(args, '--pattern');
    const language = parseArg(args, '--lang');
    const path = parseArg(args, '--path');
    const source = parseArg(args, '--source');
    const target = parseArg(args, '--target');

    // Validate required arguments
    if (!pattern) {
      throw new Error(
        '--pattern is required. Example: --pattern="function $NAME($$$ARGS)"'
      );
    }

    if (!language) {
      throw new Error('--lang is required. Example: --lang=typescript');
    }

    if (isRefactor) {
      // Refactor mode
      if (!source) {
        throw new Error(
          '--source is required for refactor. Example: --source=src/file.ts'
        );
      }
      if (!target) {
        throw new Error(
          '--target is required for refactor. Example: --target=src/new-file.ts'
        );
      }

      const result = await astGrepRefactor(pattern, language, source, target);

      return JSON.stringify(
        {
          success: result.success,
          movedCode: result.movedCode,
          sourceFile: result.sourceFile,
          targetFile: result.targetFile,
        },
        null,
        2
      );
    } else {
      // Search mode
      const searchPaths = path ? [path] : ['.'];

      const results = await astGrepSearch(pattern, language, searchPaths);

      if (results.length === 0) {
        return 'No matches found';
      }

      // Format as file:line:column:text
      const outputLines = results.map(m => {
        const firstLine = m.text.split('\n')[0];
        return `${m.file}:${m.line}:${m.column}:${firstLine}`;
      });

      return outputLines.join('\n');
    }
  },

  getHelpConfig() {
    return {
      name: 'ast',
      description:
        'AST-based code search and refactoring using ast-grep pattern matching. Supports 23 languages: TypeScript, TSX, JavaScript, Rust, Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, PHP, Bash, HTML, CSS, JSON, YAML, Lua, Elixir, Haskell.',
      usage: 'fspec research --tool=ast [options]',
      whenToUse:
        'Use to search for code patterns by structure (not text), find functions/classes/imports, or refactor by moving code between files.',
      prerequisites: [
        'Uses native ast-grep via NAPI (no additional setup required)',
      ],
      options: [
        {
          flag: '--pattern <pattern>',
          description:
            'AST pattern to search for. Use $NAME for single node, $$$ARGS for multiple nodes. (required)',
        },
        {
          flag: '--lang <language>',
          description:
            'Programming language (typescript, tsx, javascript, rust, python, go, java, c, cpp, csharp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell). (required)',
        },
        {
          flag: '--path <path>',
          description:
            'File or directory to search (defaults to current directory)',
        },
        {
          flag: '--refactor',
          description:
            'Enable refactor mode to move matched code to a new file',
        },
        {
          flag: '--source <path>',
          description: 'Source file for refactor (required with --refactor)',
        },
        {
          flag: '--target <path>',
          description: 'Target file for refactor (required with --refactor)',
        },
      ],
      examples: [
        {
          command: '--pattern="function $NAME" --lang=typescript --path=src/',
          description: 'Find all function declarations in TypeScript files',
        },
        {
          command:
            '--pattern="async function $NAME" --lang=typescript --path=src/',
          description: 'Find async function declarations',
        },
        {
          command:
            '--pattern="const $NAME" --lang=typescript --path=src/utils/',
          description: 'Find all const declarations in a directory',
        },
        {
          command:
            '--pattern="interface $NAME" --lang=typescript --path=src/types/',
          description: 'Find all TypeScript interfaces',
        },
        {
          command: '--pattern="class $NAME" --lang=typescript --path=src/',
          description: 'Find all class declarations',
        },
        {
          command: '--pattern="await $EXPR" --lang=typescript --path=src/',
          description: 'Find all await expressions',
        },
        {
          command:
            '--pattern="import $BINDING from \'$MODULE\'" --lang=typescript --path=src/',
          description: 'Find import statements',
        },
        {
          command:
            '--pattern="describe($TITLE, $CALLBACK)" --lang=typescript --path=src/',
          description: 'Find test describe blocks',
        },
        {
          command: '--pattern="fn $NAME" --lang=rust --path=src/',
          description: 'Find Rust function declarations',
        },
        {
          command: '--pattern="pub struct $NAME" --lang=rust --path=src/',
          description: 'Find Rust public struct definitions',
        },
        {
          command: '--pattern="def $NAME" --lang=python --path=.',
          description: 'Find Python function definitions',
        },
        {
          command: '--pattern="func $NAME" --lang=go --path=.',
          description: 'Find Go function declarations',
        },
        {
          command:
            '--pattern="<$COMPONENT />" --lang=tsx --path=src/components/',
          description: 'Find self-closing JSX/TSX components',
        },
        {
          command:
            '--refactor --pattern="const SafeTextInput" --lang=tsx --source=src/BigFile.tsx --target=src/SafeTextInput.tsx',
          description: 'Move a component to its own file',
        },
      ],
      features: [
        'Pattern-based AST search using ast-grep syntax',
        '$NAME matches single AST node, $$$ARGS matches zero or more nodes',
        'Output format: file:line:column:matched_text (first line only)',
        'Refactor mode moves matched code between files',
        'Supports 23 languages via native Rust implementation',
        'Respects .gitignore when searching directories',
      ],
      commonErrors: [
        {
          error: '--pattern is required',
          fix: 'Provide pattern like --pattern="function $NAME"',
        },
        {
          error: '--lang is required',
          fix: 'Provide language like --lang=typescript',
        },
        {
          error: 'Pattern matched N nodes. Refactor requires exactly 1 match.',
          fix: 'Make pattern more specific to match exactly one code block',
        },
        {
          error: 'Unsupported language',
          fix: 'Use one of: typescript, tsx, javascript, rust, python, go, java, c, cpp, csharp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell',
        },
      ],
      exitCodes: [
        { code: 0, description: 'Success' },
        { code: 1, description: 'Missing required argument' },
        {
          code: 2,
          description: 'Pattern matched wrong number of nodes (refactor mode)',
        },
      ],
      notes: [
        'Pattern syntax uses ast-grep format: $NAME for single node, $$$ARGS for multiple',
        'Patterns must match valid AST structure for the target language',
        'Some patterns like "export interface $NAME" may not work because export and interface are separate AST nodes - use "interface $NAME" instead',
        'For complex patterns, test with a single file first before searching directories',
      ],
    };
  },
};

export default tool;
