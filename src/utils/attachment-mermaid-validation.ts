import { readFile } from 'fs/promises';
import { extname } from 'path';
import { validateMermaidSyntax } from './mermaid-validation';

/**
 * Check if a file should be validated as Mermaid based on extension
 */
export function shouldValidateMermaid(filePath: string): boolean {
  const ext = extname(filePath).toLowerCase();
  return ext === '.mmd' || ext === '.mermaid' || ext === '.md';
}

/**
 * Extract Mermaid code blocks from markdown content
 * Returns array of Mermaid code blocks found in the file
 */
export function extractMermaidFromMarkdown(content: string): string[] {
  const mermaidBlocks: string[] = [];
  const regex = /```mermaid\n([\s\S]*?)\n```/g;
  let match;

  while ((match = regex.exec(content)) !== null) {
    mermaidBlocks.push(match[1]);
  }

  return mermaidBlocks;
}

/**
 * Validate Mermaid content in attachment file
 * Handles .mmd, .mermaid (direct Mermaid), and .md (extract code blocks)
 */
export async function validateMermaidAttachment(
  filePath: string
): Promise<{ valid: boolean; error?: string }> {
  const ext = extname(filePath).toLowerCase();
  const content = await readFile(filePath, 'utf-8');

  if (ext === '.md') {
    // Extract Mermaid code blocks from markdown
    const mermaidBlocks = extractMermaidFromMarkdown(content);

    if (mermaidBlocks.length === 0) {
      // No Mermaid blocks found, file is valid (it's just markdown)
      return { valid: true };
    }

    // Validate all Mermaid blocks
    for (let i = 0; i < mermaidBlocks.length; i++) {
      const result = await validateMermaidSyntax(mermaidBlocks[i]);
      if (!result.valid) {
        return {
          valid: false,
          error: `Mermaid code block ${i + 1} is invalid: ${result.error}`,
        };
      }
    }

    return { valid: true };
  } else {
    // For .mmd and .mermaid, validate entire file content as Mermaid
    return await validateMermaidSyntax(content);
  }
}
