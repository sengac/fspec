/**
 * Feature: spec/features/file-search-popup-uses-worktree-path-for-isolated-sessions.feature
 *
 * Tests for file search popup worktree path resolution (GIT-033).
 * Verifies that file search uses the correct path based on session isolation state.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from 'ink-testing-library';
import { Text } from 'ink';
import { useFileSearchInput, type UseFileSearchInputResult } from '../useFileSearchInput';

// Track calls to callGlobTool
let capturedGlobPath: string | undefined;
let mockGlobResult = { success: true, data: '' };

// Mock the toolIntegration module (which wraps globSearch)
vi.mock('../../../utils/toolIntegration', () => ({
  callGlobTool: vi.fn(async (_pattern: string, path?: string) => {
    capturedGlobPath = path;
    return mockGlobResult;
  }),
}));

// Mock the NAPI modules
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetEffectiveCwd: vi.fn(),
}));

// Mock the logger to prevent console output
vi.mock('../../../utils/logger', () => ({
  logger: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

import { sessionGetEffectiveCwd } from '@sengac/codelet-napi';

const mockSessionGetEffectiveCwd = vi.mocked(sessionGetEffectiveCwd);

// Shared state ref for testing
let hookState: UseFileSearchInputResult | null = null;
let lastOnInputChange: ReturnType<typeof vi.fn> | null = null;

// Test component with controlled state
interface TestComponentProps {
  inputValue: string;
  sessionId?: string;
  disabled?: boolean;
}

function TestComponent({ inputValue, sessionId, disabled = false }: TestComponentProps) {
  const onInputChange = vi.fn();
  lastOnInputChange = onInputChange;
  
  const fileSearch = useFileSearchInput({
    inputValue,
    onInputChange,
    terminalWidth: 120,
    disabled,
    sessionId,
  });

  // Trigger input change on mount/update to show popup
  React.useEffect(() => {
    fileSearch.handleInputChange(inputValue);
  }, [inputValue]);

  // Store current state
  hookState = fileSearch;

  return (
    <Text>
      V:{String(fileSearch.isVisible)}|F:{fileSearch.filter}|C:{fileSearch.files.length}
    </Text>
  );
}

describe('Feature: File Search Popup Uses Worktree Path for Isolated Sessions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hookState = null;
    lastOnInputChange = null;
    capturedGlobPath = undefined;
    mockGlobResult = { success: true, data: '' };
  });

  afterEach(() => {
    cleanup();
  });

  describe('Scenario: File search in isolated session finds files AI created in worktree', () => {
    it('should search worktree path when session is isolated', async () => {
      // @step Given I have an active isolated session "abc-123"
      const sessionId = 'abc-123';
      
      // @step And sessionGetEffectiveCwd("abc-123") returns ".fspec/worktrees/abc-123/"
      mockSessionGetEffectiveCwd.mockReturnValue('.fspec/worktrees/abc-123/');
      
      // @step And the AI has created "src/newfile.ts" in the worktree
      mockGlobResult = { success: true, data: 'src/newfile.ts' };
      
      // @step When I type "@newfile" in the input field
      render(<TestComponent inputValue="@newfile" sessionId={sessionId} />);
      await new Promise(r => setTimeout(r, 50));
      
      // @step Then the file search popup should show "src/newfile.ts"
      expect(hookState?.files).toHaveLength(1);
      expect(hookState?.files[0]?.path).toBe('src/newfile.ts');
      
      // @step And the glob search should have used path ".fspec/worktrees/abc-123/"
      expect(capturedGlobPath).toBe('.fspec/worktrees/abc-123/');
    });
  });

  describe('Scenario: File search in isolated session does NOT show files AI deleted', () => {
    it('should not show files that exist in main project but not in worktree', async () => {
      // @step Given I have an active isolated session "abc-123"
      const sessionId = 'abc-123';
      
      // @step And sessionGetEffectiveCwd("abc-123") returns ".fspec/worktrees/abc-123/"
      mockSessionGetEffectiveCwd.mockReturnValue('.fspec/worktrees/abc-123/');
      
      // @step And "src/legacy.js" exists in the main project but was deleted from the worktree
      // (the glob search in worktree will return empty since file doesn't exist there)
      mockGlobResult = { success: true, data: '' };
      
      // @step When I type "@legacy" in the input field
      render(<TestComponent inputValue="@legacy" sessionId={sessionId} />);
      await new Promise(r => setTimeout(r, 50));
      
      // @step Then the file search popup should NOT show "src/legacy.js"
      expect(hookState?.files).toHaveLength(0);
    });
  });

  describe('Scenario: File search reflects worktree filesystem state', () => {
    it('should show files from worktree state, not main project', async () => {
      // @step Given I have an active isolated session "abc-123"
      const sessionId = 'abc-123';
      
      // @step And sessionGetEffectiveCwd("abc-123") returns ".fspec/worktrees/abc-123/"
      mockSessionGetEffectiveCwd.mockReturnValue('.fspec/worktrees/abc-123/');
      
      // @step And the AI has modified "package.json" in the worktree
      mockGlobResult = { success: true, data: 'package.json' };
      
      // @step When I type "@package" in the input field
      render(<TestComponent inputValue="@package" sessionId={sessionId} />);
      await new Promise(r => setTimeout(r, 50));
      
      // @step Then the file search popup should show "package.json"
      expect(hookState?.files).toHaveLength(1);
      expect(hookState?.files[0]?.path).toBe('package.json');
      
      // @step And the search results should reflect the worktree's current state
      expect(capturedGlobPath).toBe('.fspec/worktrees/abc-123/');
    });
  });

  describe('Scenario: File search in non-isolated session searches project root', () => {
    it('should search project root when session is not isolated', async () => {
      // @step Given I have an active non-isolated session "def-456"
      const sessionId = 'def-456';
      
      // @step And sessionGetEffectiveCwd("def-456") returns the project root path
      const projectRoot = '/Users/test/projects/myapp';
      mockSessionGetEffectiveCwd.mockReturnValue(projectRoot);
      
      // @step And "src/config.ts" exists in the project root
      mockGlobResult = { success: true, data: 'src/config.ts' };
      
      // @step When I type "@config" in the input field
      render(<TestComponent inputValue="@config" sessionId={sessionId} />);
      await new Promise(r => setTimeout(r, 50));
      
      // @step Then the file search popup should show "src/config.ts"
      expect(hookState?.files).toHaveLength(1);
      expect(hookState?.files[0]?.path).toBe('src/config.ts');
      
      // @step And the glob search should have used the project root path
      expect(capturedGlobPath).toBe(projectRoot);
    });
  });

  describe('Scenario: File search before session creation uses project root fallback', () => {
    it('should use project root when sessionId is null', async () => {
      // @step Given no session has been created yet
      // @step And sessionId is null
      // (sessionId is undefined/null)
      
      // @step And "README.md" exists in the project root
      mockGlobResult = { success: true, data: 'README.md' };
      
      // @step When I type "@README" in the input field
      render(<TestComponent inputValue="@README" sessionId={undefined} />);
      await new Promise(r => setTimeout(r, 50));
      
      // @step Then the file search popup should show "README.md"
      expect(hookState?.files).toHaveLength(1);
      expect(hookState?.files[0]?.path).toBe('README.md');
      
      // @step And the glob search should have used the project root path
      // (undefined path = current working directory / project root)
      expect(capturedGlobPath).toBeUndefined();
    });
  });

  describe('Scenario: File search falls back to project root when sessionGetEffectiveCwd returns null', () => {
    it('should fall back to project root when NAPI returns null', async () => {
      // @step Given I have an active session "ghi-789"
      const sessionId = 'ghi-789';
      
      // @step But sessionGetEffectiveCwd("ghi-789") returns null
      mockSessionGetEffectiveCwd.mockReturnValue(null);
      
      // @step And "src/app.ts" exists in the project root
      mockGlobResult = { success: true, data: 'src/app.ts' };
      
      // @step When I type "@app" in the input field
      render(<TestComponent inputValue="@app" sessionId={sessionId} />);
      await new Promise(r => setTimeout(r, 50));
      
      // @step Then the file search popup should show "src/app.ts"
      expect(hookState?.files).toHaveLength(1);
      expect(hookState?.files[0]?.path).toBe('src/app.ts');
      
      // @step And the glob search should have used the project root path
      // (undefined path = fallback to project root)
      expect(capturedGlobPath).toBeUndefined();
    });
  });

  describe('Edge cases', () => {
    it('should not call sessionGetEffectiveCwd when sessionId is not provided', async () => {
      mockGlobResult = { success: true, data: '' };
      
      render(<TestComponent inputValue="@test" sessionId={undefined} />);
      await new Promise(r => setTimeout(r, 50));
      
      expect(mockSessionGetEffectiveCwd).not.toHaveBeenCalled();
    });

    it('should call sessionGetEffectiveCwd with correct sessionId', async () => {
      mockSessionGetEffectiveCwd.mockReturnValue('/some/path');
      mockGlobResult = { success: true, data: '' };
      
      render(<TestComponent inputValue="@test" sessionId="my-session-123" />);
      await new Promise(r => setTimeout(r, 50));
      
      expect(mockSessionGetEffectiveCwd).toHaveBeenCalledWith('my-session-123');
    });

    it('should update search path when sessionId changes', async () => {
      mockGlobResult = { success: true, data: '' };
      
      // First session
      mockSessionGetEffectiveCwd.mockReturnValue('/worktree/session-1');
      const { rerender, unmount } = render(<TestComponent inputValue="@test" sessionId="session-1" />);
      await new Promise(r => setTimeout(r, 50));
      
      expect(capturedGlobPath).toBe('/worktree/session-1');
      
      // Unmount and remount with different session (simulating session switch)
      // This is how the TUI actually works - components remount on session switch
      unmount();
      capturedGlobPath = undefined;
      
      mockSessionGetEffectiveCwd.mockReturnValue('/worktree/session-2');
      render(<TestComponent inputValue="@test" sessionId="session-2" />);
      await new Promise(r => setTimeout(r, 50));
      
      expect(capturedGlobPath).toBe('/worktree/session-2');
    });
  });
});
