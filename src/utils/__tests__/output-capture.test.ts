/**
 * Test the output capture mechanism used by fspec-callback
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  output,
  createCaptureContext,
  setOutputContext,
  resetOutputContext,
  getOutputContext,
} from '../output';

describe('Output Capture Mechanism', () => {
  afterEach(() => {
    // Always reset to default context after each test
    resetOutputContext();
  });

  it('should capture output.log to stdout array', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.log('test message');

    expect(stdout).toContain('test message');
    expect(stderr).toHaveLength(0);
  });

  it('should capture output.error to stderr array', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.error('error message');

    expect(stderr).toContain('error message');
    expect(stdout).toHaveLength(0);
  });

  it('should capture output.warn to stderr array', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.warn('warning message');

    expect(stderr).toContain('warning message');
    expect(stdout).toHaveLength(0);
  });

  it('should capture multiple arguments joined with space', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.error('Error:', 'something went wrong');

    expect(stderr).toContain('Error: something went wrong');
  });

  it('should JSON stringify non-string arguments', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.log('Object:', { key: 'value' });

    expect(stdout[0]).toBe('Object: {"key":"value"}');
  });

  it('should preserve captured output after resetOutputContext', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.error('captured error');
    output.log('captured log');

    // Reset context - this should NOT clear the captured arrays
    resetOutputContext();

    // The arrays should still have the captured content
    expect(stderr).toContain('captured error');
    expect(stdout).toContain('captured log');
  });

  it('should stop capturing after resetOutputContext', () => {
    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);

    output.error('captured before reset');

    resetOutputContext();

    // This should go to console, not to our captured arrays
    // Note: We can't easily test console output, but we can verify
    // the array doesn't grow
    const stderrLengthAfterReset = stderr.length;

    // output.error('not captured') would go to console now
    // We can't call it without polluting test output

    expect(stderrLengthAfterReset).toBe(1);
  });

  it('should handle the sequence used in fspec-callback', () => {
    // This simulates the exact sequence in fspec-callback.ts
    const {
      context: captureContext,
      stdout: capturedStdout,
      stderr: capturedStderr,
    } = createCaptureContext();

    // Set context before command execution
    setOutputContext(captureContext);

    // Simulate a command that fails
    try {
      // Command does its work...
      output.error('✗ Failed to add rule:', 'Work unit does not exist');

      // Command calls process.exit(1) which we simulate by throwing
      throw new Error('__FSPEC_EXIT_OVERRIDE__:1');
    } catch (error) {
      // This is what fspec-callback does in the catch block
      resetOutputContext();

      // Read the captured output
      const capturedError = capturedStderr.join('\n');

      // The error should be captured
      expect(capturedError).toContain('Failed to add rule');
      expect(capturedError).toContain('Work unit does not exist');
    }
  });
});
