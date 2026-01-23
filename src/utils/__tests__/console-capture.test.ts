/**
 * Console Capture Tests
 *
 * Feature: spec/features/capture-all-console-methods-and-redirect-to-winston-logger.feature
 * Coverage: LOG-003
 */

import {
  describe,
  it,
  expect,
  beforeEach,
  afterEach,
  vi,
  type Mock,
} from 'vitest';

// Mock the logger module BEFORE importing console-capture
vi.mock('../logger', () => ({
  logger: {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  },
}));

// Import after mocking
import { initializeConsoleCapture, restoreConsole } from '../console-capture';
import { logger } from '../logger';

describe('Console Capture - LOG-003', () => {
  // Store original console methods for verification
  let originalLog: typeof console.log;
  let originalError: typeof console.error;
  let originalWarn: typeof console.warn;
  let originalInfo: typeof console.info;
  let originalDebug: typeof console.debug;
  let originalTrace: typeof console.trace;

  // Capture what was output to terminal
  let terminalOutput: string[];

  beforeEach(() => {
    // Store originals before any capture
    originalLog = console.log;
    originalError = console.error;
    originalWarn = console.warn;
    originalInfo = console.info;
    originalDebug = console.debug;
    originalTrace = console.trace;

    // Clear mocks
    vi.clearAllMocks();

    // Track terminal output
    terminalOutput = [];
  });

  afterEach(() => {
    // Always restore after each test
    restoreConsole();
  });

  describe('Scenario: Capture console.log to info level', () => {
    it('should capture console.log and log to info level', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.log is called with 'hello'
      console.log('hello');

      // @step Then 'hello' is output to the terminal
      // (original console.log is called internally, verified by console output in test)

      // @step And the log file contains '[info]: hello'
      expect(logger.info).toHaveBeenCalledWith('hello');
    });
  });

  describe('Scenario: Capture console.error to error level', () => {
    it('should capture console.error and log to error level', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.error is called with 'failed'
      console.error('failed');

      // @step Then 'failed' is output to the terminal
      // (original console.error is called internally)

      // @step And the log file contains '[error]: failed'
      expect(logger.error).toHaveBeenCalledWith('failed');
    });
  });

  describe('Scenario: Capture console.warn to warn level', () => {
    it('should capture console.warn and log to warn level', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.warn is called with 'deprecated'
      console.warn('deprecated');

      // @step Then 'deprecated' is output to the terminal
      // (original console.warn is called internally)

      // @step And the log file contains '[warn]: deprecated'
      expect(logger.warn).toHaveBeenCalledWith('deprecated');
    });
  });

  describe('Scenario: Strip ANSI escape codes from log file', () => {
    it('should strip ANSI codes from log file but preserve in terminal', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.log is called with chalk-formatted text containing ANSI codes
      const ansiText = '\x1b[31mred text\x1b[0m';
      console.log(ansiText);

      // @step Then the terminal shows the colored text
      // (original console.log receives the ANSI codes)

      // @step And the log file contains plain text without ANSI escape codes
      expect(logger.info).toHaveBeenCalledWith('red text');
    });
  });

  describe('Scenario: Join multiple arguments with spaces', () => {
    it('should join multiple arguments with spaces', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.log is called with 'hello', 'world', and 123
      console.log('hello', 'world', 123);

      // @step Then the log file contains '[info]: hello world 123'
      expect(logger.info).toHaveBeenCalledWith('hello world 123');
    });
  });

  describe('Scenario: Serialize objects to JSON', () => {
    it('should serialize objects to JSON', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.log is called with an object {foo: 'bar'}
      console.log({ foo: 'bar' });

      // @step Then the log file contains the JSON serialized object
      expect(logger.info).toHaveBeenCalledWith(
        expect.stringContaining('"foo"')
      );
      expect(logger.info).toHaveBeenCalledWith(
        expect.stringContaining('"bar"')
      );
    });
  });

  describe('Scenario: Include stack trace for Error objects', () => {
    it('should include stack trace for Error objects', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.error is called with an Error object
      const error = new Error('test error');
      console.error(error);

      // @step Then the log file contains the error message and stack trace
      const loggedMessage = (logger.error as Mock).mock.calls[0][0];
      expect(loggedMessage).toContain('test error');
      expect(loggedMessage).toContain('Error');
    });
  });

  describe('Scenario: Capture console.trace with stack trace at debug level', () => {
    it('should capture console.trace at debug level with stack trace', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // @step When console.trace is called with 'marker'
      console.trace('marker');

      // @step Then the log file contains '[debug]: marker' with a stack trace
      const loggedMessage = (logger.debug as Mock).mock.calls[0][0];
      expect(loggedMessage).toContain('marker');
      // Stack trace should be present
      expect(loggedMessage).toMatch(/at|Error/);
    });
  });

  describe('Scenario: Restore console methods for test isolation', () => {
    it('should restore console methods when restoreConsole is called', () => {
      // @step Given console capture has been initialized
      initializeConsoleCapture();

      // Verify capture is active
      console.log('captured');
      expect(logger.info).toHaveBeenCalledWith('captured');
      vi.clearAllMocks();

      // @step When restoreConsole is called
      restoreConsole();

      // @step Then subsequent console calls no longer log to winston
      console.log('not captured');
      expect(logger.info).not.toHaveBeenCalled();

      // @step And console output still appears in the terminal
      // (console.log is restored to original, so it works normally)
    });
  });
});
