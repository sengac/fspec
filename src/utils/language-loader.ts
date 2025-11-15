/**
 * Language Loader - Lazy Loading Tree-Sitter Parsers
 *
 * Dynamically imports tree-sitter language parsers on-demand to improve CLI startup time.
 * Implements caching to avoid re-importing the same parser multiple times.
 */

// Parser cache to avoid re-importing same language
const parserCache = new Map<string, unknown>();

/**
 * Load a tree-sitter language parser dynamically
 *
 * @param language - Language name (e.g., 'javascript', 'typescript', 'python')
 * @returns Promise resolving to the language parser module
 */
export async function loadLanguageParser(language: string): Promise<unknown> {
  // Check cache first
  if (parserCache.has(language)) {
    return parserCache.get(language)!;
  }

  let parser: unknown;

  switch (language) {
    case 'javascript':
      parser = (await import('@sengac/tree-sitter-javascript')).default;
      break;

    case 'typescript':
      parser = (await import('@sengac/tree-sitter-typescript')).default
        .typescript;
      break;

    case 'tsx':
      parser = (await import('@sengac/tree-sitter-typescript')).default.tsx;
      break;

    case 'python':
      parser = (await import('@sengac/tree-sitter-python')).default;
      break;

    case 'go':
      parser = (await import('@sengac/tree-sitter-go')).default;
      break;

    case 'rust':
      parser = (await import('@sengac/tree-sitter-rust')).default;
      break;

    case 'java':
      parser = (await import('@sengac/tree-sitter-java')).default;
      break;

    case 'ruby':
      parser = (await import('@sengac/tree-sitter-ruby')).default;
      break;

    case 'csharp':
      parser = (await import('@sengac/tree-sitter-c-sharp')).default;
      break;

    case 'php':
      parser = (await import('@sengac/tree-sitter-php')).default.php;
      break;

    case 'cpp':
      parser = (await import('@sengac/tree-sitter-cpp')).default;
      break;

    case 'c':
      parser = (await import('@sengac/tree-sitter-c')).default;
      break;

    case 'bash':
      parser = (await import('@sengac/tree-sitter-bash')).default;
      break;

    case 'json':
      parser = (await import('@sengac/tree-sitter-json')).default;
      break;

    case 'kotlin':
      parser = (await import('@sengac/tree-sitter-kotlin')).default;
      break;

    case 'dart':
      parser = (await import('@sengac/tree-sitter-dart')).default;
      break;

    case 'swift':
      parser = (await import('@sengac/tree-sitter-swift')).default;
      break;

    default:
      throw new Error(`Unsupported language: ${language}`);
  }

  // Cache for future use
  parserCache.set(language, parser);

  return parser;
}
