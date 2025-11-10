/**
 * AST Parser using tree-sitter
 * RES-014: Language-Agnostic AST Tool for AI Analysis
 */

import Parser from 'tree-sitter';
import JavaScript from 'tree-sitter-javascript';
import TypeScript from 'tree-sitter-typescript';
import Python from 'tree-sitter-python';
import Go from 'tree-sitter-go';
import Rust from 'tree-sitter-rust';
import Java from 'tree-sitter-java';
import Ruby from 'tree-sitter-ruby';
import CSharp from 'tree-sitter-c-sharp';
import Php from 'tree-sitter-php';
import Cpp from 'tree-sitter-cpp';
import Bash from 'tree-sitter-bash';
import Json from 'tree-sitter-json';
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
function getLanguageParser(
  filePath: string
): { parser: Parser; language: string } | null {
  const ext = extname(filePath);
  const parser = new Parser();

  switch (ext) {
    case '.js':
    case '.jsx':
      parser.setLanguage(JavaScript);
      return { parser, language: 'javascript' };
    case '.ts':
      parser.setLanguage(TypeScript.typescript);
      return { parser, language: 'typescript' };
    case '.tsx':
      parser.setLanguage(TypeScript.tsx);
      return { parser, language: 'typescript' };
    case '.py':
      parser.setLanguage(Python);
      return { parser, language: 'python' };
    case '.go':
      parser.setLanguage(Go);
      return { parser, language: 'go' };
    case '.rs':
      parser.setLanguage(Rust);
      return { parser, language: 'rust' };
    case '.java':
      parser.setLanguage(Java);
      return { parser, language: 'java' };
    case '.rb':
      parser.setLanguage(Ruby);
      return { parser, language: 'ruby' };
    case '.cs':
      parser.setLanguage(CSharp);
      return { parser, language: 'csharp' };
    case '.php':
      parser.setLanguage(Php.php);
      return { parser, language: 'php' };
    case '.cpp':
    case '.cc':
    case '.cxx':
    case '.c++':
    case '.hpp':
    case '.hh':
    case '.hxx':
    case '.h++':
      parser.setLanguage(Cpp);
      return { parser, language: 'cpp' };
    case '.sh':
    case '.bash':
      parser.setLanguage(Bash);
      return { parser, language: 'bash' };
    case '.json':
      parser.setLanguage(Json);
      return { parser, language: 'json' };
    default:
      return null;
  }
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
function findImports(node: Parser.SyntaxNode, filePath: string): ASTMatch {
  const imports: Array<{
    lineNumber: number;
    symbols: string[];
    path?: string;
  }> = [];

  function traverse(n: Parser.SyntaxNode) {
    if (n.type === 'import_statement' || n.type === 'import_declaration') {
      const symbols: string[] = [];

      // Extract imported symbols
      for (const child of n.children) {
        if (child.type === 'import_specifier' || child.type === 'identifier') {
          symbols.push(child.text);
        }
      }

      imports.push({
        lineNumber: n.startPosition.row + 1,
        symbols,
        path: undefined, // Could extract from string_literal if needed
      });
    }

    for (const child of n.children) {
      traverse(child);
    }
  }

  traverse(node);

  return {
    filePath,
    imports,
  };
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
  const parserInfo = getLanguageParser(filePath);
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
    return findImports(tree.rootNode, filePath);
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
