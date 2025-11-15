/**
 * AST Parser using tree-sitter
 * RES-014: Language-Agnostic AST Tool for AI Analysis
 */

import Parser from '@sengac/tree-sitter';
import { loadLanguageParser } from './language-loader';
import { readFile } from 'fs/promises';
import { extname } from 'path';

export interface ASTMatch {
  filePath?: string;
  lineNumber?: number;
  startLine?: number;
  endLine?: number;
  code?: string;
  className?: string;
  functionName?: string;
  nodeType?: string;
  symbols?: string[];
  imports?: Array<{
    lineNumber: number;
    symbols: string[];
    path?: string;
  }>;
  classes?: Array<{
    name: string;
    methods: Array<{
      name: string;
      startLine: number;
      endLine: number;
    }>;
  }>;
  exports?: Array<{
    filePath: string;
    startLine: number;
    endLine: number;
    code: string;
  }>;
  grammar?: string;
  matches?: ASTMatch[];
}

/**
 * Get language parser based on file extension
 */
async function getLanguageParser(
  filePath: string
): Promise<{ parser: Parser; language: string } | null> {
  const ext = extname(filePath);
  const parser = new Parser();

  let language: string;

  switch (ext) {
    case '.js':
    case '.jsx':
      language = 'javascript';
      break;
    case '.ts':
      language = 'typescript';
      break;
    case '.tsx':
      language = 'tsx';
      break;
    case '.py':
      language = 'python';
      break;
    case '.go':
      language = 'go';
      break;
    case '.rs':
      language = 'rust';
      break;
    case '.java':
      language = 'java';
      break;
    case '.rb':
      language = 'ruby';
      break;
    case '.cs':
      language = 'csharp';
      break;
    case '.php':
      language = 'php';
      break;
    case '.cpp':
    case '.cc':
    case '.cxx':
    case '.c++':
    case '.hpp':
    case '.hh':
    case '.hxx':
    case '.h++':
      language = 'cpp';
      break;
    case '.sh':
    case '.bash':
      language = 'bash';
      break;
    case '.json':
      language = 'json';
      break;
    default:
      return null;
  }

  // Lazy load the language parser
  const languageModule = await loadLanguageParser(language);
  parser.setLanguage(languageModule);

  return { parser, language };
}

/**
 * Find all function definitions in AST
 */
function findFunctions(node: Parser.SyntaxNode, filePath: string): ASTMatch[] {
  const matches: ASTMatch[] = [];

  function traverse(n: Parser.SyntaxNode) {
    // JavaScript/TypeScript function declarations
    if (
      n.type === 'function_declaration' ||
      n.type === 'function' ||
      n.type === 'arrow_function' ||
      n.type === 'method_definition'
    ) {
      matches.push({
        filePath,
        lineNumber: n.startPosition.row + 1,
        startLine: n.startPosition.row + 1,
        endLine: n.endPosition.row + 1,
        code: n.text,
        nodeType: n.type,
      });
    }

    // Python function definitions
    if (n.type === 'function_definition') {
      matches.push({
        filePath,
        lineNumber: n.startPosition.row + 1,
        startLine: n.startPosition.row + 1,
        endLine: n.endPosition.row + 1,
        code: n.text,
        nodeType: n.type,
      });
    }

    // Go function declarations
    if (n.type === 'function_declaration' || n.type === 'method_declaration') {
      matches.push({
        filePath,
        lineNumber: n.startPosition.row + 1,
        startLine: n.startPosition.row + 1,
        endLine: n.endPosition.row + 1,
        code: n.text,
        nodeType: n.type,
      });
    }

    for (const child of n.children) {
      traverse(child);
    }
  }

  traverse(node);
  return matches;
}

/**
 * Find specific class by name
 */
function findClass(
  node: Parser.SyntaxNode,
  className: string,
  filePath: string
): ASTMatch | null {
  function traverse(n: Parser.SyntaxNode): ASTMatch | null {
    if (n.type === 'class_declaration' || n.type === 'class') {
      // Find the class name identifier
      const nameNode = n.children.find(
        c => c.type === 'identifier' || c.type === 'type_identifier'
      );
      if (nameNode && nameNode.text === className) {
        return {
          className,
          filePath,
          startLine: n.startPosition.row + 1,
          endLine: n.endPosition.row + 1,
          code: n.text,
        };
      }
    }

    for (const child of n.children) {
      const result = traverse(child);
      if (result) return result;
    }
    return null;
  }

  return traverse(node);
}

/**
 * Find all class declarations
 */
function findAllClasses(node: Parser.SyntaxNode, filePath: string): ASTMatch[] {
  const matches: ASTMatch[] = [];

  function traverse(n: Parser.SyntaxNode) {
    if (n.type === 'class_declaration' || n.type === 'class') {
      // Find the class name identifier
      const nameNode = n.children.find(
        c => c.type === 'identifier' || c.type === 'type_identifier'
      );
      const className = nameNode?.text || 'Anonymous';

      matches.push({
        className,
        filePath,
        lineNumber: n.startPosition.row + 1,
        startLine: n.startPosition.row + 1,
        endLine: n.endPosition.row + 1,
        code: n.text,
        nodeType: n.type,
      });
    }

    for (const child of n.children) {
      traverse(child);
    }
  }

  traverse(node);
  return matches;
}

/**
 * Find all import statements
 */
function findImports(node: Parser.SyntaxNode, filePath: string): ASTMatch[] {
  const matches: ASTMatch[] = [];

  function traverse(n: Parser.SyntaxNode) {
    if (n.type === 'import_statement' || n.type === 'import_declaration') {
      const symbols: string[] = [];
      let importPath = '';

      // Extract imported symbols and path
      for (const child of n.children) {
        if (child.type === 'import_specifier' || child.type === 'identifier') {
          symbols.push(child.text);
        }
        if (child.type === 'string' || child.type === 'string_fragment') {
          importPath = child.text.replace(/['"]/g, '');
        }
      }

      matches.push({
        filePath,
        lineNumber: n.startPosition.row + 1,
        startLine: n.startPosition.row + 1,
        endLine: n.endPosition.row + 1,
        code: n.text,
        nodeType: n.type,
        symbols,
        imports: [
          {
            lineNumber: n.startPosition.row + 1,
            symbols,
            path: importPath || undefined,
          },
        ],
      });
    }

    for (const child of n.children) {
      traverse(child);
    }
  }

  traverse(node);
  return matches;
}

/**
 * Find all classes and their methods (Python)
 */
function findPythonClasses(node: Parser.SyntaxNode): ASTMatch {
  const classes: Array<{
    name: string;
    methods: Array<{
      name: string;
      startLine: number;
      endLine: number;
    }>;
  }> = [];

  function traverse(n: Parser.SyntaxNode) {
    if (n.type === 'class_definition') {
      const nameNode = n.children.find(c => c.type === 'identifier');
      const className = nameNode?.text || 'Unknown';

      const methods: Array<{
        name: string;
        startLine: number;
        endLine: number;
      }> = [];

      // Find methods within class
      function findMethods(classNode: Parser.SyntaxNode) {
        for (const child of classNode.children) {
          if (child.type === 'function_definition') {
            const methodNameNode = child.children.find(
              c => c.type === 'identifier'
            );
            if (methodNameNode) {
              methods.push({
                name: methodNameNode.text,
                startLine: child.startPosition.row + 1,
                endLine: child.endPosition.row + 1,
              });
            }
          }
          findMethods(child);
        }
      }

      findMethods(n);

      classes.push({
        name: className,
        methods,
      });
    }

    for (const child of n.children) {
      traverse(child);
    }
  }

  traverse(node);

  return {
    classes,
    grammar: 'python',
  };
}

/**
 * Find all export default statements
 */
function findExportDefaults(
  node: Parser.SyntaxNode,
  filePath: string
): ASTMatch {
  const exports: Array<{
    filePath: string;
    startLine: number;
    endLine: number;
    code: string;
  }> = [];

  function traverse(n: Parser.SyntaxNode) {
    if (n.type === 'export_statement' || n.type === 'export_declaration') {
      // Check if it's a default export
      const hasDefault = n.children.some(c => c.text === 'default');
      if (hasDefault) {
        exports.push({
          filePath,
          startLine: n.startPosition.row + 1,
          endLine: n.endPosition.row + 1,
          code: n.text,
        });
      }
    }

    for (const child of n.children) {
      traverse(child);
    }
  }

  traverse(node);

  return {
    filePath,
    exports,
  };
}

/**
 * Parse a file and extract AST information based on query
 */
export async function parseFile(
  filePath: string,
  query?: string
): Promise<ASTMatch> {
  const parserInfo = await getLanguageParser(filePath);
  if (!parserInfo) {
    throw new Error(`Unsupported file type: ${filePath}`);
  }

  const { parser, language } = parserInfo;
  const sourceCode = await readFile(filePath, 'utf-8');
  const tree = parser.parse(sourceCode);

  // If no query, return all functions
  if (!query) {
    const functions = findFunctions(tree.rootNode, filePath);
    return {
      matches: functions,
    };
  }

  const queryLower = query.toLowerCase();

  // Check for "find all" or "all" patterns
  const isFindAll =
    queryLower.includes('find all') ||
    queryLower.includes('all ') ||
    queryLower.startsWith('all');

  // Class search
  if (queryLower.includes('class')) {
    if (isFindAll) {
      // Find all classes
      const classes = findAllClasses(tree.rootNode, filePath);
      return {
        matches: classes,
      };
    } else {
      // Check for specific class name pattern
      const classMatch = /class\s+(\w+)/.exec(query);
      if (classMatch) {
        const className = classMatch[1];
        const result = findClass(tree.rootNode, className, filePath);
        if (result) {
          return result;
        }
      } else {
        // Default to finding all classes when no specific name
        const classes = findAllClasses(tree.rootNode, filePath);
        return {
          matches: classes,
        };
      }
    }
  }

  // Function definitions
  if (queryLower.includes('function')) {
    const functions = findFunctions(tree.rootNode, filePath);
    return {
      matches: functions,
    };
  }

  // Import statements
  if (queryLower.includes('import')) {
    const imports = findImports(tree.rootNode, filePath);
    return {
      matches: imports,
    };
  }

  // Python classes with methods
  if (language === 'python' && queryLower.includes('class')) {
    return findPythonClasses(tree.rootNode);
  }

  // Export default
  if (queryLower.includes('export default')) {
    return findExportDefaults(tree.rootNode, filePath);
  }

  // Default: return all functions
  const functions = findFunctions(tree.rootNode, filePath);
  return {
    matches: functions,
  };
}
