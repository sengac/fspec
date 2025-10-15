import { JSDOM } from 'jsdom';

export interface MermaidValidationResult {
  valid: boolean;
  error?: string;
}

/**
 * Validate Mermaid diagram syntax using mermaid.parse() with jsdom
 */
export async function validateMermaidSyntax(
  code: string
): Promise<MermaidValidationResult> {
  try {
    // Create a jsdom instance with a minimal HTML document
    const dom = new JSDOM('<!DOCTYPE html><html><body></body></html>', {
      runScripts: 'dangerously',
      resources: 'usable',
    });

    // Set up global DOM objects for mermaid
    const { window } = dom;
    const originalWindow = global.window;
    const originalDocument = global.document;

    global.window = window as any;
    global.document = window.document as any;
    Object.defineProperty(global, 'navigator', {
      value: window.navigator,
      configurable: true,
    });

    // Dynamically import mermaid after setting up the DOM
    const mermaid = (await import('mermaid')).default;

    // Initialize mermaid with minimal config
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
    });

    // Use mermaid.parse() to validate syntax
    await mermaid.parse(code);

    // Clean up globals
    if (originalWindow) {
      global.window = originalWindow;
    } else {
      delete (global as any).window;
    }
    if (originalDocument) {
      global.document = originalDocument;
    } else {
      delete (global as any).document;
    }
    delete (global as any).navigator;

    return { valid: true };
  } catch (error: any) {
    // Clean up globals even on error
    delete (global as any).window;
    delete (global as any).document;
    delete (global as any).navigator;

    return {
      valid: false,
      error: error.message || 'Unknown Mermaid syntax error',
    };
  }
}
