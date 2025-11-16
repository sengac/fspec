import { describe, it, expect } from 'vitest';
import { pascalCaseToSentence } from '../text-formatting';

describe('pascalCaseToSentence', () => {
  it('should convert PascalCase to lowercase sentence', () => {
    expect(pascalCaseToSentence('UserAuthenticated')).toBe(
      'user authenticated'
    );
  });

  it('should convert camelCase to lowercase sentence', () => {
    expect(pascalCaseToSentence('userLoggedIn')).toBe('user logged in');
  });

  it('should handle single word', () => {
    expect(pascalCaseToSentence('User')).toBe('user');
  });

  it('should handle lowercase input', () => {
    expect(pascalCaseToSentence('user')).toBe('user');
  });

  it('should handle multiple consecutive capitals', () => {
    expect(pascalCaseToSentence('SendWelcomeEmail')).toBe('send welcome email');
  });

  it('should handle empty string', () => {
    expect(pascalCaseToSentence('')).toBe('');
  });

  it('should handle string with spaces (returns trimmed lowercase)', () => {
    expect(pascalCaseToSentence('User Authenticated')).toBe(
      'user  authenticated'
    );
  });

  it('should handle acronyms', () => {
    expect(pascalCaseToSentence('HTTPRequest')).toBe('h t t p request');
  });
});
