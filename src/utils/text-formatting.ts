/**
 * Text formatting utilities
 * Provides common text transformation functions for consistent formatting
 */

/**
 * Convert PascalCase or camelCase text to lowercase sentence with spaces
 *
 * @example
 * pascalCaseToSentence('UserAuthenticated') // 'user authenticated'
 * pascalCaseToSentence('SendWelcomeEmail') // 'send welcome email'
 * pascalCaseToSentence('userLoggedIn') // 'user logged in'
 *
 * @param text - PascalCase or camelCase string to convert
 * @returns Lowercase sentence with spaces between words
 */
export function pascalCaseToSentence(text: string): string {
  return text
    .replace(/([A-Z])/g, ' $1')
    .trim()
    .toLowerCase();
}
