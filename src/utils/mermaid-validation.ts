import { JSDOM } from 'jsdom';

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
  const subgraphMatch = code.match(/subgraph\s+(\S+?)(?:\s*\[|\s|$)/);
  if (subgraphMatch) {
    const subgraphId = subgraphMatch[1];
    // Check if ID contains invalid characters (anything other than alphanumeric, underscore, hyphen)
    if (!/^[a-zA-Z0-9_-]+$/.test(subgraphId)) {
      return {
        valid: false,
        error:
          'Invalid subgraph identifier. Use only letters, numbers, underscores, and hyphens',
      };
    }
  }

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
    const originalWindow = globalThis.window;
    const originalDocument = globalThis.document;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    globalThis.window = window as any;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    globalThis.document = window.document as any;
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
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      delete (globalThis as any).window;
    }
    if (originalDocument) {
      globalThis.document = originalDocument;
    } else {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      delete (globalThis as any).document;
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (globalThis as any).navigator;

    return { valid: true };
  } catch (error: unknown) {
    // Clean up globals even on error
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (globalThis as any).window;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (globalThis as any).document;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (globalThis as any).navigator;

    const errorMessage =
      error instanceof Error ? error.message : 'Unknown Mermaid syntax error';
    return {
      valid: false,
      error: errorMessage,
    };
  }
}
