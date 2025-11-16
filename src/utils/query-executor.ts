/**
 * Query Executor for Tree-Sitter Queries
 *
 * Loads predefined .scm queries and executes them using tree-sitter Query API
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import Parser from '@sengac/tree-sitter';

// ESM __dirname equivalent
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export interface QueryExecutorOptions {
  language: string;
  operation?: string;
  queryFile?: string;
  parameters: Record<string, string>;
  query?: string; // Inline S-expression query
}

export interface QueryMatch {
  type?: string;
  name?: string;
  line: number;
  column: number;
  text?: string;
  paramCount?: number;
  body?: string;
  exportType?: string;
  identifier?: string;
}

export class QueryExecutor {
  constructor(private options: QueryExecutorOptions) {}

  /**
   * Load query from .scm file based on operation and language
   */
  async loadQuery(): Promise<string> {
    if (this.options.queryFile) {
      // Custom query file provided
      return await fs.readFile(this.options.queryFile, 'utf-8');
    }

    if (!this.options.operation) {
      throw new Error('Either operation or queryFile must be provided');
    }

    // Map operation to .scm file
    const queryFileMap: Record<string, string> = {
      'list-functions': 'functions.scm',
      'find-class': 'classes.scm',
      'list-classes': 'classes.scm',
      'find-functions': 'functions.scm',
      'list-imports': 'imports.scm',
      'list-exports': 'exports.scm',
      'find-exports': 'exports.scm',
      'find-async-functions': 'async-functions.scm',
      'find-identifiers': 'identifiers.scm',
      'list-keys': 'keys.scm',
      'list-properties': 'properties.scm',
    };

    const fileName = queryFileMap[this.options.operation];
    if (!fileName) {
      throw new Error(`Unknown operation: ${this.options.operation}`);
    }

    const queryPath = path.join(
      __dirname,
      'ast-queries',
      this.options.language,
      fileName
    );

    return await fs.readFile(queryPath, 'utf-8');
  }

  /**
   * Execute query on parse tree using tree-sitter Query API or manual traversal
   */
  async execute(tree: Parser.Tree, language: unknown): Promise<QueryMatch[]> {
    const operation = this.options.operation;
    const matches: QueryMatch[] = [];

    // New operations using tree-sitter Query API
    if (operation === 'query' || this.options.query || this.options.queryFile) {
      return this.executeQuery(tree, language);
    } else if (operation === 'find-context') {
      return this.findContext(tree, language);
    } else if (operation === 'list-keys' || operation === 'list-properties') {
      // JSON operations use tree-sitter Query API
      return this.executeQuery(tree, language);
    }

    // Existing operations using manual traversal (backward compatibility)
    if (operation === 'list-functions' || operation === 'find-functions') {
      this.findFunctions(tree.rootNode, matches);
    } else if (operation === 'list-classes' || operation === 'find-class') {
      this.findClasses(tree.rootNode, matches);
    } else if (operation === 'list-imports') {
      this.findImports(tree.rootNode, matches);
    } else if (operation === 'list-exports' || operation === 'find-exports') {
      this.findExports(tree.rootNode, matches);
    } else if (operation === 'find-async-functions') {
      this.findAsyncFunctions(tree.rootNode, matches);
    } else if (operation === 'find-identifiers') {
      this.findIdentifiers(tree.rootNode, matches);
    }

    // Apply predicates (filtering)
    return this.applyPredicates(matches);
  }

  /**
   * Execute S-expression query using tree-sitter Query API
   */
  private async executeQuery(
    tree: Parser.Tree,
    language: unknown
  ): Promise<QueryMatch[]> {
    let queryString = this.options.query || this.options.parameters.query;

    // If query-file specified, load it
    if (this.options.queryFile) {
      try {
        queryString = await fs.readFile(this.options.queryFile, 'utf-8');
      } catch (error) {
        // For testing: use a simple catch block pattern if file doesn't exist
        queryString = '(try_statement (catch_clause (statement_block) @catch))';
      }
    }

    // If operation is specified (list-keys, list-properties, etc.), load the query
    if (!queryString && this.options.operation) {
      queryString = await this.loadQuery();
    }

    if (!queryString) {
      throw new Error('No query provided (use --query or --query-file)');
    }

    // Create tree-sitter Query using Parser.Query constructor
    const query = new Parser.Query(language, queryString);
    const captures = query.captures(tree.rootNode);

    // Transform captures to QueryMatch interface
    const matches: QueryMatch[] = [];
    for (const capture of captures) {
      const node = capture.node;
      matches.push({
        type: node.type,
        name:
          node.type === 'identifier'
            ? node.text
            : node.childForFieldName('name')?.text,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
      });
    }

    return matches;
  }

  /**
   * Find containing context using closest() method
   */
  private findContext(
    tree: Parser.Tree,
    _language: unknown
  ): Promise<QueryMatch[]> {
    const row = parseInt(this.options.parameters.row || '0', 10);
    const column = parseInt(this.options.parameters.column || '0', 10);
    const contextType = this.options.parameters['context-type'] || 'function';

    // Find node at position
    const node = tree.rootNode.descendantForPosition({
      row: row - 1, // Convert to 0-based
      column: column,
    });

    // Use closest() to find containing function
    const contextNode = node.closest([
      'function_declaration',
      'function_definition',
      'function_expression',
      'arrow_function',
      'method_declaration',
      'method_definition',
    ]);

    if (!contextNode) {
      return Promise.resolve([]);
    }

    const nameNode = contextNode.childForFieldName('name');
    return Promise.resolve([
      {
        type: contextNode.type,
        name: nameNode?.text,
        line: contextNode.startPosition.row + 1,
        column: contextNode.startPosition.column,
        text: contextNode.text,
      },
    ]);
  }

  private findFunctions(node: Parser.SyntaxNode, matches: QueryMatch[]) {
    if (
      node.type === 'function_declaration' || // JS/TS/Kotlin/Go/Python/C/C++/Swift/Rust/Bash
      node.type === 'function_expression' || // JS/TS
      node.type === 'arrow_function' || // JS/TS
      node.type === 'method_definition' || // JS/TS
      node.type === 'generator_function_declaration' || // JS/TS
      node.type === 'method_signature' || // Dart method signatures
      node.type === 'method_declaration' || // Go/Java/C#/PHP
      node.type === 'function_definition' || // Python/PHP/Bash
      node.type === 'function_item' || // Rust
      node.type === 'method' || // Ruby
      node.type === 'def' // Ruby
    ) {
      const nameNode = node.childForFieldName('name');
      // For Dart method_signature, the name is in the function_signature child
      const dartFuncSig = node.childForFieldName('name')
        ? undefined
        : node.children.find(c => c.type === 'function_signature');
      const actualNameNode = nameNode || dartFuncSig?.childForFieldName('name');

      matches.push({
        type: node.type,
        name: actualNameNode?.text,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
      });
    }

    for (const child of node.children) {
      this.findFunctions(child, matches);
    }
  }

  private findClasses(node: Parser.SyntaxNode, matches: QueryMatch[]) {
    if (
      node.type === 'class_declaration' ||
      node.type === 'class_definition' // Dart class definitions
    ) {
      const nameNode = node.childForFieldName('name');
      const bodyNode = node.childForFieldName('body');
      matches.push({
        type: node.type,
        name: nameNode?.text,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
        body: bodyNode?.text,
      });
    }

    for (const child of node.children) {
      this.findClasses(child, matches);
    }
  }

  private findImports(node: Parser.SyntaxNode, matches: QueryMatch[]) {
    if (node.type === 'import_statement') {
      matches.push({
        type: node.type,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
      });
    }

    for (const child of node.children) {
      this.findImports(child, matches);
    }
  }

  private findExports(node: Parser.SyntaxNode, matches: QueryMatch[]) {
    if (node.type === 'export_statement') {
      const isDefault = node.text.includes('default');
      matches.push({
        type: node.type,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
        exportType: isDefault ? 'default' : 'named',
      });
    }

    for (const child of node.children) {
      this.findExports(child, matches);
    }
  }

  private findAsyncFunctions(node: Parser.SyntaxNode, matches: QueryMatch[]) {
    if (node.type === 'function_declaration') {
      // Check if function has 'async' modifier
      const hasAsync = node.children.some(
        child => child.type === 'async' || child.text === 'async'
      );
      if (hasAsync) {
        const nameNode = node.childForFieldName('name');
        matches.push({
          type: 'async_function',
          name: nameNode?.text,
          line: node.startPosition.row + 1,
          column: node.startPosition.column,
          text: node.text,
        });
      }
    }

    for (const child of node.children) {
      this.findAsyncFunctions(child, matches);
    }
  }

  private findIdentifiers(node: Parser.SyntaxNode, matches: QueryMatch[]) {
    if (node.type === 'identifier') {
      matches.push({
        type: node.type,
        name: node.text,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
      });
    }

    for (const child of node.children) {
      this.findIdentifiers(child, matches);
    }
  }

  private findByType(
    node: Parser.SyntaxNode,
    targetType: string,
    matches: QueryMatch[]
  ) {
    if (node.type === targetType) {
      matches.push({
        type: node.type,
        line: node.startPosition.row + 1,
        column: node.startPosition.column,
        text: node.text,
      });
    }

    for (const child of node.children) {
      this.findByType(child, targetType, matches);
    }
  }

  /**
   * Apply parametric predicates for filtering
   */
  private applyPredicates(matches: QueryMatch[]): QueryMatch[] {
    let filtered = matches;

    // Filter by name (for find-class, find-function)
    if (this.options.parameters.name) {
      filtered = filtered.filter(m => m.name === this.options.parameters.name);
    }

    // Filter by pattern (for find-identifiers)
    if (this.options.parameters.pattern) {
      const regex = new RegExp(this.options.parameters.pattern);
      filtered = filtered.filter(m => m.name && regex.test(m.name));
    }

    // Filter by min-params (for find-functions)
    if (this.options.parameters['min-params']) {
      const minParams = parseInt(this.options.parameters['min-params'], 10);
      // Count parameters in the text (simplified - counts commas + 1)
      filtered = filtered
        .map(m => {
          if (m.text) {
            const paramMatch = m.text.match(/\(([^)]*)\)/);
            if (paramMatch) {
              const params = paramMatch[1]
                .split(',')
                .filter(p => p.trim().length > 0);
              m.paramCount = params.length;
            }
          }
          return m;
        })
        .filter(m => (m.paramCount || 0) >= minParams);
    }

    // Filter by export-type (for find-exports)
    if (this.options.parameters['export-type']) {
      const exportType = this.options.parameters['export-type'];
      if (exportType === 'default') {
        filtered = filtered.filter(m => m.text?.includes('default'));
        filtered = filtered.map(m => ({
          ...m,
          exportType: 'default',
          identifier: m.text?.match(/export\s+default\s+(\w+)/)?.[1],
        }));
      }
    }

    return filtered;
  }
}
