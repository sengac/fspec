import type {
  GherkinDocument,
  Feature,
  FeatureChild,
  Scenario,
  Background,
  Step,
  TableRow,
  Tag,
  Examples,
  Rule,
  DocString,
  DataTable,
  Comment,
} from '@cucumber/messages';

interface FormatOptions {
  printWidth: number;
  indent: string;
}

const DEFAULT_OPTIONS: FormatOptions = {
  printWidth: 80,
  indent: '  ',
};

/**
 * GherkinFormatter - converts Gherkin AST back to formatted text
 *
 * Core principle: STABILITY - formatting the same AST twice produces identical output
 *
 * Rules:
 * - Preserve all content from AST exactly
 * - Apply consistent indentation
 * - Don't wrap or modify text
 * - Don't add/remove blank lines beyond what's in the AST
 */
export class GherkinFormatter {
  private options: FormatOptions;

  constructor(options: Partial<FormatOptions> = {}) {
    this.options = { ...DEFAULT_OPTIONS, ...options };
  }

  format(ast: GherkinDocument): string {
    const lines: string[] = [];

    if (!ast.feature) {
      return '';
    }

    // Build comment map indexed by line number
    const commentMap = this.buildCommentMap(ast.comments || []);

    this.formatFeature(ast.feature, lines, commentMap);

    // Ensure file ends with single newline
    return lines.join('\n') + '\n';
  }

  /**
   * Build a map of comments indexed by line number
   */
  private buildCommentMap(comments: readonly Comment[]): Map<number, string> {
    const map = new Map<number, string>();
    comments.forEach(comment => {
      map.set(comment.location.line, comment.text);
    });
    return map;
  }

  /**
   * Insert comments that appear before the given line number
   *
   * Note: Comments from the Gherkin parser already include their indentation
   * as part of the comment text, so we output them as-is without adding indent.
   */
  private insertCommentsBeforeLine(
    currentLine: number,
    commentMap: Map<number, string>,
    lines: string[]
  ): void {
    const sortedCommentLines = Array.from(commentMap.keys()).sort(
      (a, b) => a - b
    );

    for (const commentLine of sortedCommentLines) {
      if (commentLine < currentLine) {
        const commentText = commentMap.get(commentLine);
        if (commentText !== undefined) {
          // Comments include their own indentation, so output as-is
          lines.push(commentText);
          commentMap.delete(commentLine); // Remove processed comment
        }
      }
    }
  }

  private formatFeature(
    feature: Feature,
    lines: string[],
    commentMap: Map<number, string>
  ): void {
    // Insert comments before tags
    if (feature.tags.length > 0) {
      this.insertCommentsBeforeLine(
        feature.tags[0].location.line,
        commentMap,
        lines
      );
    } else {
      this.insertCommentsBeforeLine(feature.location.line, commentMap, lines);
    }

    // Tags - each on separate line
    if (feature.tags.length > 0) {
      feature.tags.forEach(tag => {
        lines.push(tag.name);
      });
    }

    // Insert comments before Feature keyword
    this.insertCommentsBeforeLine(feature.location.line, commentMap, lines);

    // Feature keyword and name
    lines.push(`Feature: ${feature.name}`);

    // Description (free-form text, may include """ delimiters as literal text)
    if (feature.description) {
      this.formatDescription(feature.description, lines, 1);
    }

    // Feature children (Background, Scenarios, Rules)
    feature.children.forEach((child, index) => {
      // Always add blank line before children
      if (index === 0) {
        lines.push(''); // Blank line after feature/description
      } else {
        lines.push(''); // Blank line between children
      }

      // Insert comments before child element
      const childLocation =
        child.background?.location ||
        child.scenario?.location ||
        child.rule?.location;
      if (childLocation) {
        this.insertCommentsBeforeLine(childLocation.line, commentMap, lines);
      }

      this.formatFeatureChild(child, lines, 0, commentMap);
    });
  }

  private formatFeatureChild(
    child: FeatureChild,
    lines: string[],
    baseIndent: number,
    commentMap: Map<number, string>
  ): void {
    if (child.background) {
      this.formatBackground(child.background, lines, baseIndent, commentMap);
    } else if (child.scenario) {
      this.formatScenario(child.scenario, lines, baseIndent, commentMap);
    } else if (child.rule) {
      this.formatRule(child.rule, lines, baseIndent, commentMap);
    }
  }

  private formatBackground(
    background: Background,
    lines: string[],
    baseIndent: number,
    commentMap: Map<number, string>
  ): void {
    const indent = this.getIndent(baseIndent + 1);

    lines.push(`${indent}Background: ${background.name}`);

    if (background.description) {
      this.formatDescription(background.description, lines, baseIndent + 2);
    }

    background.steps.forEach(step => {
      this.formatStep(step, lines, baseIndent + 2, commentMap);
    });
  }

  private formatScenario(
    scenario: Scenario,
    lines: string[],
    baseIndent: number,
    commentMap: Map<number, string>
  ): void {
    const indent = this.getIndent(baseIndent + 1);

    // Tags
    if (scenario.tags.length > 0) {
      scenario.tags.forEach(tag => {
        lines.push(`${indent}${tag.name}`);
      });
    }

    // Scenario keyword (can be "Scenario" or "Scenario Outline")
    lines.push(`${indent}${scenario.keyword}: ${scenario.name}`);

    // Description
    if (scenario.description) {
      this.formatDescription(scenario.description, lines, baseIndent + 2);
    }

    // Steps
    scenario.steps.forEach(step => {
      this.formatStep(step, lines, baseIndent + 2, commentMap);
    });

    // Examples (for Scenario Outline)
    scenario.examples.forEach(examples => {
      lines.push(''); // Blank line before Examples
      this.formatExamples(examples, lines, baseIndent + 2, commentMap);
    });
  }

  private formatRule(
    rule: Rule,
    lines: string[],
    baseIndent: number,
    commentMap: Map<number, string>
  ): void {
    const indent = this.getIndent(baseIndent + 1);

    // Tags
    if (rule.tags.length > 0) {
      rule.tags.forEach(tag => {
        lines.push(`${indent}${tag.name}`);
      });
    }

    lines.push(`${indent}Rule: ${rule.name}`);

    // Description
    if (rule.description) {
      this.formatDescription(rule.description, lines, baseIndent + 2);
    }

    // Rule children
    rule.children.forEach((child, index) => {
      if (index > 0 || rule.description) {
        lines.push(''); // Blank line before each child
      }
      this.formatFeatureChild(child, lines, baseIndent + 1, commentMap);
    });
  }

  private formatStep(
    step: Step,
    lines: string[],
    indentLevel: number,
    commentMap: Map<number, string>
  ): void {
    const indent = this.getIndent(indentLevel);

    // Insert comments before step
    this.insertCommentsBeforeLine(step.location.line, commentMap, lines);

    // Steps are output as-is (no wrapping)
    // The keyword includes trailing space (e.g., "Given ")
    lines.push(`${indent}${step.keyword}${step.text}`);

    // DocString
    if (step.docString) {
      this.formatDocString(step.docString, lines, indentLevel + 1);
    }

    // DataTable
    if (step.dataTable) {
      this.formatDataTable(step.dataTable, lines, indentLevel + 1);
    }
  }

  private formatExamples(
    examples: Examples,
    lines: string[],
    indentLevel: number,
    commentMap: Map<number, string>
  ): void {
    const indent = this.getIndent(indentLevel);

    // Tags
    if (examples.tags.length > 0) {
      examples.tags.forEach(tag => {
        lines.push(`${indent}${tag.name}`);
      });
    }

    // Examples keyword
    lines.push(`${indent}${examples.keyword}:`);

    // Description
    if (examples.description) {
      this.formatDescription(examples.description, lines, indentLevel + 1);
    }

    // Table
    if (examples.tableHeader && examples.tableBody) {
      const allRows = [examples.tableHeader, ...examples.tableBody];
      this.formatTableRows(allRows, lines, indentLevel + 1);
    }
  }

  private formatDocString(
    docString: DocString,
    lines: string[],
    indentLevel: number
  ): void {
    const indent = this.getIndent(indentLevel);
    const delimiter = docString.delimiter; // """ or ```

    // Opening delimiter with optional media type
    if (docString.mediaType) {
      lines.push(`${indent}${delimiter}${docString.mediaType}`);
    } else {
      lines.push(`${indent}${delimiter}`);
    }

    // Content - split by newlines and preserve as-is
    if (docString.content) {
      const contentLines = docString.content.split('\n');
      contentLines.forEach(line => {
        if (line.trim() === '') {
          lines.push('');
        } else {
          lines.push(`${indent}${line}`);
        }
      });
    }

    // Closing delimiter
    lines.push(`${indent}${delimiter}`);
  }

  private formatDataTable(
    dataTable: DataTable,
    lines: string[],
    indentLevel: number
  ): void {
    this.formatTableRows(dataTable.rows, lines, indentLevel);
  }

  private formatTableRows(
    rows: readonly TableRow[],
    lines: string[],
    indentLevel: number
  ): void {
    if (rows.length === 0) return;

    const indent = this.getIndent(indentLevel);

    // Calculate max width for each column
    const columnWidths: number[] = [];
    rows.forEach(row => {
      row.cells.forEach((cell, colIndex) => {
        const width = cell.value.length;
        columnWidths[colIndex] = Math.max(columnWidths[colIndex] || 0, width);
      });
    });

    // Format each row with aligned columns
    rows.forEach(row => {
      const cells = row.cells.map((cell, colIndex) => {
        return cell.value.padEnd(columnWidths[colIndex]);
      });
      lines.push(`${indent}| ${cells.join(' | ')} |`);
    });
  }

  /**
   * Format description text
   *
   * Description is free-form text from the AST. It may contain:
   * - Literal """ or ``` characters (not DocString delimiters at feature level)
   * - Multiple lines with varying indentation
   * - Blank lines for section separation
   *
   * We output it exactly as the parser gives it, with consistent base indentation.
   */
  private formatDescription(
    description: string,
    lines: string[],
    indentLevel: number
  ): void {
    const indent = this.getIndent(indentLevel);
    let consecutiveBlankLines = 0;

    // Split by newlines and limit consecutive blank lines to max 2
    description.split('\n').forEach(line => {
      const trimmed = line.trim();

      if (trimmed === '') {
        // Blank line - preserve up to 2 consecutive blank lines
        if (consecutiveBlankLines < 2) {
          lines.push('');
          consecutiveBlankLines++;
        }
      } else {
        // Non-blank line - reset counter and add indented line
        consecutiveBlankLines = 0;
        lines.push(`${indent}${trimmed}`);
      }
    });
  }

  private getIndent(level: number): string {
    return this.options.indent.repeat(level);
  }
}

export function formatGherkinDocument(ast: GherkinDocument): string {
  const formatter = new GherkinFormatter();
  return formatter.format(ast);
}
