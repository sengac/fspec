import { JSDOM } from 'jsdom';

// TypeScript global declarations to reduce 'as any' usage
declare global {
  // eslint-disable-next-line no-undef
  var window: Window & typeof globalThis;
  // eslint-disable-next-line no-undef
  var document: Document;
  // eslint-disable-next-line no-undef
  var navigator: Navigator;
}

export interface MermaidValidationResult {
  valid: boolean;
  error?: string;
}

/**
 * Validate Mermaid diagram syntax and semantics using mermaid.render() with jsdom
 */
export async function validateMermaidSyntax(
  code: string
): Promise<MermaidValidationResult> {
  // Pre-render validation: Check for known problematic patterns that may render but fail in browser
  const quotedSubgraphPattern = /subgraph\s+"[^"]+"/;
  if (quotedSubgraphPattern.test(code)) {
    return {
      valid: false,
      error:
        'Quoted subgraph titles are not supported. Use: subgraph ID[Title]',
    };
  }

  // Check for invalid subgraph identifiers (special characters in the ID part before optional [Title])
  // Valid: subgraph ID or subgraph ID[Title] where ID contains only letters, numbers, underscores, hyphens
  // Invalid: subgraph ID!!! or subgraph ID@#$ (contains special chars)
  // Use matchAll() to check ALL subgraphs, not just the first one
  const subgraphMatches = code.matchAll(/subgraph\s+(\S+?)(?:\s*\[|\s|$)/g);
  for (const match of subgraphMatches) {
    const subgraphId = match[1];
    // Check if ID contains invalid characters (anything other than alphanumeric, underscore, hyphen)
    if (!/^[a-zA-Z0-9_-]+$/.test(subgraphId)) {
      return {
        valid: false,
        error: `Invalid subgraph identifier '${subgraphId}'. Use only letters, numbers, underscores, and hyphens`,
      };
    }
  }

  // Store original globals before try block so they're accessible in catch
  const originalWindow = globalThis.window;
  const originalDocument = globalThis.document;

  try {
    // Create a jsdom instance with a minimal HTML document
    const dom = new JSDOM(
      '<!DOCTYPE html><html><body><div id="mermaid-container"></div></body></html>',
      {
        runScripts: 'dangerously',
        resources: 'usable',
      }
    );

    // Set up global DOM objects for mermaid
    const { window } = dom;

    // eslint-disable-next-line no-undef
    globalThis.window = window as Window & typeof globalThis;
    // eslint-disable-next-line no-undef
    globalThis.document = window.document as Document;
    Object.defineProperty(globalThis, 'navigator', {
      value: window.navigator,
      configurable: true,
    });

    // Mock SVG DOM APIs that mermaid.render() requires but JSDOM doesn't provide
    if (!window.SVGElement.prototype.getBBox) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (window.SVGElement.prototype as any).getBBox = function () {
        return { x: 0, y: 0, width: 100, height: 100 };
      };
    }
    if (!window.SVGElement.prototype.getComputedTextLength) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (window.SVGElement.prototype as any).getComputedTextLength = function () {
        return 100;
      };
    }

    // Dynamically import mermaid after setting up the DOM
    const mermaid = (await import('mermaid')).default;

    // Initialize mermaid with minimal config
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
    });

    // Use mermaid.render() to validate both syntax and semantics
    await mermaid.render('validation-diagram', code);

    // Clean up globals
    if (originalWindow) {
      globalThis.window = originalWindow;
    } else {
      delete globalThis.window;
    }
    if (originalDocument) {
      globalThis.document = originalDocument;
    } else {
      delete globalThis.document;
    }
    delete globalThis.navigator;

    return { valid: true };
  } catch (error: unknown) {
    // Clean up globals even on error (match success path logic)
    if (originalWindow) {
      globalThis.window = originalWindow;
    } else {
      delete globalThis.window;
    }
    if (originalDocument) {
      globalThis.document = originalDocument;
    } else {
      delete globalThis.document;
    }
    delete globalThis.navigator;

    const errorMessage =
      error instanceof Error ? error.message : 'Unknown Mermaid syntax error';
    return {
      valid: false,
      error: errorMessage,
    };
  }
}
