/**
 * Tests for stderr rendering in tool output
 *
 * Feature: Stderr output from bash commands should be rendered in red
 *
 * Tests the stderr marker system that enables TypeScript to identify
 * and render stderr content from bash commands in red.
 *
 * Data flow:
 * 1. Rust bash tool marks stderr lines with ⚠stderr⚠ prefix
 * 2. NAPI passes the marked content to TypeScript
 * 3. TypeScript rendering detects marker and renders in red
 */

import { describe, it, expect } from 'vitest';

// The stderr marker constant - must match Rust (codelet/tools/src/bash.rs)
const STDERR_MARKER = '⚠stderr⚠';

/**
 * Helper to simulate the TypeScript stderr detection logic from AgentView.tsx
 */
function detectStderrLines(
  content: string
): { line: string; isStderr: boolean }[] {
  return content.split('\n').map(line => {
    if (line.includes(STDERR_MARKER)) {
      return {
        line: line.replace(new RegExp(STDERR_MARKER, 'g'), ''),
        isStderr: true,
      };
    }
    return { line, isStderr: false };
  });
}

/**
 * Helper to simulate the TypeScript streaming stderr marker addition from AgentView.tsx
 */
function markStderrChunk(chunk: string, isStderr: boolean): string {
  if (!isStderr) return chunk;
  return chunk
    .split('\n')
    .map(line => (line ? `${STDERR_MARKER}${line}` : line))
    .join('\n');
}

describe('Feature: Stderr Rendering in Tool Output', () => {
  describe('Scenario: Rust stderr marker detection', () => {
    it('should detect single stderr line', () => {
      const content = `${STDERR_MARKER}This is an error message`;
      const lines = detectStderrLines(content);

      expect(lines).toHaveLength(1);
      expect(lines[0].isStderr).toBe(true);
      expect(lines[0].line).toBe('This is an error message');
    });

    it('should detect multiple stderr lines', () => {
      const content = [
        `${STDERR_MARKER}Error line 1`,
        `${STDERR_MARKER}Error line 2`,
        `${STDERR_MARKER}Error line 3`,
      ].join('\n');

      const lines = detectStderrLines(content);

      expect(lines).toHaveLength(3);
      expect(lines.every(l => l.isStderr)).toBe(true);
      expect(lines[0].line).toBe('Error line 1');
      expect(lines[1].line).toBe('Error line 2');
      expect(lines[2].line).toBe('Error line 3');
    });

    it('should distinguish stdout from stderr', () => {
      const content = [
        'stdout line 1',
        `${STDERR_MARKER}stderr line`,
        'stdout line 2',
      ].join('\n');

      const lines = detectStderrLines(content);

      expect(lines).toHaveLength(3);
      expect(lines[0].isStderr).toBe(false);
      expect(lines[0].line).toBe('stdout line 1');
      expect(lines[1].isStderr).toBe(true);
      expect(lines[1].line).toBe('stderr line');
      expect(lines[2].isStderr).toBe(false);
      expect(lines[2].line).toBe('stdout line 2');
    });

    it('should handle content without any stderr', () => {
      const content = 'Normal output\nAnother line\nNo errors here';
      const lines = detectStderrLines(content);

      expect(lines).toHaveLength(3);
      expect(lines.every(l => !l.isStderr)).toBe(true);
    });

    it('should handle empty content', () => {
      const lines = detectStderrLines('');

      expect(lines).toHaveLength(1);
      expect(lines[0].isStderr).toBe(false);
      expect(lines[0].line).toBe('');
    });
  });

  describe('Scenario: Streaming stderr marker addition', () => {
    it('should mark single line stderr chunk', () => {
      const chunk = 'Error message\n';
      const marked = markStderrChunk(chunk, true);

      expect(marked).toBe(`${STDERR_MARKER}Error message\n`);
    });

    it('should mark multiline stderr chunk', () => {
      const chunk = 'Error 1\nError 2\nError 3\n';
      const marked = markStderrChunk(chunk, true);

      expect(marked).toBe(
        `${STDERR_MARKER}Error 1\n${STDERR_MARKER}Error 2\n${STDERR_MARKER}Error 3\n`
      );
    });

    it('should not mark stdout chunks', () => {
      const chunk = 'Normal output\nAnother line\n';
      const marked = markStderrChunk(chunk, false);

      expect(marked).toBe(chunk);
      expect(marked).not.toContain(STDERR_MARKER);
    });

    it('should handle empty lines in stderr chunk', () => {
      const chunk = 'Error 1\n\nError 2\n';
      const marked = markStderrChunk(chunk, true);

      // Empty lines should not be marked (no content to mark)
      expect(marked).toBe(
        `${STDERR_MARKER}Error 1\n\n${STDERR_MARKER}Error 2\n`
      );
    });
  });

  describe('Scenario: End-to-end stderr flow', () => {
    it('should round-trip stderr through marking and detection', () => {
      // Simulate: Rust streams stderr → TypeScript marks it → Later detects it
      const originalStderr =
        'warning: unused variable\nerror: compilation failed\n';

      // Step 1: TypeScript marks the streaming stderr (simulating AgentView chunk handling)
      const markedContent = markStderrChunk(originalStderr, true);

      // Step 2: Later, TypeScript detects and strips markers for rendering
      const lines = detectStderrLines(markedContent);

      // Verify all lines detected as stderr
      expect(lines.filter(l => l.line.trim()).every(l => l.isStderr)).toBe(
        true
      );

      // Verify original content preserved (markers stripped)
      const reconstructed = lines.map(l => l.line).join('\n');
      expect(reconstructed).toBe(originalStderr);
    });

    it('should handle mixed stdout/stderr from final tool result', () => {
      // Simulate: Rust bash tool combines stdout + marked stderr
      const bashOutput = [
        'Build started...',
        'Compiling main.rs',
        `${STDERR_MARKER}warning: unused import`,
        `${STDERR_MARKER}error: cannot find value`,
        'Build complete',
      ].join('\n');

      const lines = detectStderrLines(bashOutput);

      expect(lines[0]).toEqual({ line: 'Build started...', isStderr: false });
      expect(lines[1]).toEqual({ line: 'Compiling main.rs', isStderr: false });
      expect(lines[2]).toEqual({
        line: 'warning: unused import',
        isStderr: true,
      });
      expect(lines[3]).toEqual({
        line: 'error: cannot find value',
        isStderr: true,
      });
      expect(lines[4]).toEqual({ line: 'Build complete', isStderr: false });
    });

    it('should handle command failure with marked stderr', () => {
      // Simulate: Rust returns error with marked stderr
      const errorOutput = [
        'Command failed with exit code 1',
        `${STDERR_MARKER}error: unknown command 'foobar'`,
        `${STDERR_MARKER}(Did you mean 'foo'?)`,
      ].join('\n');

      const lines = detectStderrLines(errorOutput);

      // First line is error message (not stderr)
      expect(lines[0]).toEqual({
        line: 'Command failed with exit code 1',
        isStderr: false,
      });
      // Subsequent lines are marked stderr
      expect(lines[1].isStderr).toBe(true);
      expect(lines[2].isStderr).toBe(true);
    });
  });

  describe('Scenario: Marker constant consistency', () => {
    it('should use the correct marker string', () => {
      // This must match the Rust constant in codelet/tools/src/bash.rs
      expect(STDERR_MARKER).toBe('⚠stderr⚠');
    });

    it('should be distinguishable from normal content', () => {
      // The marker should not appear in normal text
      const normalContent = 'stderr output here'; // word "stderr" without marker
      const lines = detectStderrLines(normalContent);

      expect(lines[0].isStderr).toBe(false);
      expect(lines[0].line).toBe('stderr output here');
    });

    it('should handle marker-like but incomplete patterns', () => {
      // Edge case: partial marker should not trigger detection
      const content = '⚠ this is a warning';
      const lines = detectStderrLines(content);

      expect(lines[0].isStderr).toBe(false);
    });
  });
});
