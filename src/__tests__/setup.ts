/**
 * Test setup for React Testing Library and component testing
 */

import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Suppress console warnings during tests

globalThis.console = {
  ...console,
  warn: vi.fn(),
  error: vi.fn(),
};
