/**
 * Test fixture helper specifically for add-diagram tests
 * Creates minimal foundation with architectureDiagrams array
 */

import type { GenericFoundation } from './foundation-fixtures';
import { createMinimalFoundation } from './foundation-fixtures';

/**
 * Creates a minimal foundation with empty architectureDiagrams array
 */
export function createFoundationWithDiagrams(
  diagrams: Array<{ title: string; mermaidCode: string; description?: string }> = []
): GenericFoundation {
  const foundation = createMinimalFoundation();
  return {
    ...foundation,
    architectureDiagrams: diagrams,
  };
}
