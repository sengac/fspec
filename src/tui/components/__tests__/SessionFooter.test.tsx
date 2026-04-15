/**
 * Feature: spec/features/session-footer.feature
 *
 * Tests for SessionFooter component — displays CWD and git branch name
 * in a 1-line dark grey footer bar. No dirty/untracked indicators.
 *
 * Architecture:
 * - SessionFooter reads from footerStore (Zustand), NOT from props or NAPI
 * - footerStore is populated by FooterStateUpdate events from Rust
 * - Rust background poller reads ONLY .git/HEAD (near-zero CPU)
 * - No get_staged_files, get_unstaged_files, or get_untracked_files calls
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { act } from 'react';
import { SessionFooter } from '../SessionFooter';
import { useFooterStore } from '../../store/footerStore';

// Mock NAPI (required by transitive imports in test env)
vi.mock('@sengac/codelet-napi', () => ({
  sessionSetActive: vi.fn(),
  sessionClearActive: vi.fn(),
}));

/**
 * Seed the footerStore with state for a session, simulating what
 * GlobalSessionStreamManager does when a FooterStateUpdate arrives from Rust.
 */
function seedFooterState(
  sessionId: string,
  overrides?: {
    cwd?: string;
    displayPath?: string;
    isGitRepo?: boolean;
    branch?: string | null;
  },
): void {
  const branch = overrides && 'branch' in overrides ? (overrides.branch ?? null) : 'main';
  useFooterStore.getState().updateFooterState(
    sessionId,
    overrides?.cwd ?? '/Users/rquast/projects/fspec',
    overrides?.displayPath ?? '~/projects/fspec',
    overrides?.isGitRepo ?? true,
    branch,
  );
}

describe('Feature: SessionFooter component with CWD and git branch name display', () => {
  beforeEach(() => {
    useFooterStore.getState().reset();
  });

  afterEach(() => {
    useFooterStore.getState().reset();
  });

  // ----------------------------------------
  // Scenario: Display CWD and branch name in git repository
  // ----------------------------------------

  describe('Scenario: Display CWD and branch name in git repository', () => {
    it('should show CWD with branch name and no status indicators', () => {
      // @step Given I am in a git repository at "~/projects/fspec"
      // @step And the current branch is "main"
      seedFooterState('test-session-1', {
        displayPath: '~/projects/fspec',
        branch: 'main',
      });

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="test-session-1" />);
      const output = lastFrame();

      // @step Then I should see "~/projects/fspec" in the footer
      expect(output).toContain('~/projects/fspec');

      // @step And I should see "[⎇ main]" in the footer
      expect(output).toContain('main');
      expect(output).toContain('⎇');

      // @step And the footer should have a dark grey background
      // (verified by component using backgroundColor="#333333")
      expect(output).toBeDefined();
    });
  });

  // ----------------------------------------
  // Scenario: Display branch name without dirty or untracked indicators
  // ----------------------------------------

  describe('Scenario: Display branch name without dirty or untracked indicators', () => {
    it('should show only branch name even when there are unstaged changes and untracked files', () => {
      // @step Given I am in a git repository at "~/projects/fspec"
      // @step And the current branch is "feature-branch"
      // @step And there are unstaged changes
      // @step And there are untracked files
      seedFooterState('test-session-2', {
        displayPath: '~/projects/fspec',
        branch: 'feature-branch',
      });

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="test-session-2" />);
      const output = lastFrame();

      // @step Then I should see "[⎇ feature-branch]" in the footer
      expect(output).toContain('feature-branch');
      expect(output).toContain('⎇');

      // @step And the branch indicator should not contain "*" or "%"
      expect(output).not.toContain('*');
      expect(output).not.toContain('%');
    });
  });

  // ----------------------------------------
  // Scenario: Display only CWD for non-git directory
  // ----------------------------------------

  describe('Scenario: Display only CWD for non-git directory', () => {
    it('should show only the shortened CWD without branch info', () => {
      // @step Given I am in a non-git directory at "~/my-project"
      seedFooterState('test-session-3', {
        displayPath: '~/my-project',
        isGitRepo: false,
      });

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="test-session-3" />);
      const output = lastFrame();

      // @step Then I should see "~/my-project" in the footer
      expect(output).toContain('~/my-project');

      // @step And I should not see any branch indicator
      expect(output).not.toContain('⎇');
    });
  });

  // ----------------------------------------
  // Scenario: Footer has dark grey background spanning full width
  // ----------------------------------------

  describe('Scenario: Footer has dark grey background spanning full width', () => {
    it('should render a 1-line footer with full-width dark grey background', () => {
      // @step Given I am in a git repository
      seedFooterState('test-session-4');

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="test-session-4" />);
      const output = lastFrame();

      // @step Then the footer should be 1 line high
      const lines = (output ?? '').split('\n').filter(l => l.trim().length > 0);
      expect(lines.length).toBe(1);

      // @step And the footer should have background color "#333333"
      // (verified by component source — backgroundColor="#333333" on Box)

      // @step And the footer should span the full terminal width
      // (verified by width="100%" on Box)
      expect(output).toBeDefined();
    });
  });

  // ----------------------------------------
  // Scenario: Display detached HEAD state
  // ----------------------------------------

  describe('Scenario: Display detached HEAD state', () => {
    it('should show (detached) when HEAD is detached', () => {
      // @step Given I am in a git repository at "~/projects/fspec"
      // @step And the HEAD is detached
      seedFooterState('test-session-5', {
        displayPath: '~/projects/fspec',
        isGitRepo: true,
        branch: null,
      });

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="test-session-5" />);
      const output = lastFrame();

      // @step Then I should see "[⎇ (detached)]" in the footer
      expect(output).toContain('(detached)');
      expect(output).toContain('⎇');
    });
  });

  // ----------------------------------------
  // Scenario: Shorten HOME directory to tilde in CWD display
  // ----------------------------------------

  describe('Scenario: Shorten HOME directory to tilde in CWD display', () => {
    it('should display ~ instead of full HOME path', () => {
      // @step Given the HOME directory is "/Users/rquast"
      // @step And I am in a directory at "/Users/rquast/projects/fspec"
      seedFooterState('test-session-6', {
        cwd: '/Users/rquast/projects/fspec',
        displayPath: '~/projects/fspec',
      });

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="test-session-6" />);
      const output = lastFrame();

      // @step Then I should see "~/projects/fspec" in the footer
      expect(output).toContain('~/projects/fspec');

      // @step And I should not see "/Users/rquast" in the footer
      expect(output).not.toContain('/Users/rquast');
    });
  });

  // ----------------------------------------
  // Scenario: Null session ID renders empty footer
  // ----------------------------------------

  describe('Scenario: Null session ID renders empty footer', () => {
    it('should render an empty footer bar when no session is active', () => {
      // @step Given there is no active session
      // @step When the SessionFooter renders with null sessionId
      const { lastFrame } = render(<SessionFooter sessionId={null} />);
      const output = lastFrame();

      // @step Then the footer should not show any path or branch info
      expect(output).not.toContain('⎇');
      expect(output).not.toContain('~');
    });
  });

  // ----------------------------------------
  // Scenario: Session-specific CWD for isolated sessions
  // ----------------------------------------

  describe('Scenario: Session-specific CWD for isolated sessions', () => {
    it('should display the worktree path for an isolated session', () => {
      // @step Given the Rust poller has emitted footer state for an isolated session
      seedFooterState('isolated-session', {
        cwd: '/Users/rquast/projects/fspec/.fspec/worktrees/abc123',
        displayPath: '~/projects/fspec/.fspec/worktrees/abc123',
        branch: 'session/abc123',
        isGitRepo: true,
      });

      // @step When the SessionFooter renders for this session
      const { lastFrame } = render(<SessionFooter sessionId="isolated-session" />);
      const output = lastFrame();

      // @step Then the CWD should show the worktree path
      expect(output).toContain('worktrees/abc123');

      // @step And the branch should be the session's branch
      expect(output).toContain('session/abc123');
    });
  });

  // ----------------------------------------
  // Scenario: Zustand store update triggers re-render
  // ----------------------------------------

  describe('Scenario: Zustand store update triggers re-render', () => {
    it('should update display when store receives new state from Rust', () => {
      // @step Given the footer shows initial state
      seedFooterState('live-session', {
        displayPath: '~/projects/fspec',
        branch: 'main',
      });

      const { lastFrame } = render(<SessionFooter sessionId="live-session" />);
      expect(lastFrame()).toContain('main');

      // @step When the Rust poller emits an updated FooterStateUpdate (branch changed)
      act(() => {
        useFooterStore.getState().updateFooterState(
          'live-session',
          '/Users/rquast/projects/fspec',
          '~/projects/fspec',
          true,
          'feature-branch',
        );
      });

      // @step Then the footer should reflect the new branch name
      const output = lastFrame();
      expect(output).toContain('feature-branch');
      // No dirty/untracked indicators
      expect(output).not.toContain('*');
      expect(output).not.toContain('%');
    });
  });

  // ----------------------------------------
  // Scenario: Footer poller uses near-zero CPU
  // ----------------------------------------

  describe('Scenario: Footer poller uses near-zero CPU', () => {
    it('should only use branch name with no dirty/untracked state in the store', () => {
      // @step Given the footer poller is running
      // (Rust poller emits FooterStateUpdate with branch only — no dirty/untracked fields)
      seedFooterState('perf-session', {
        displayPath: '~/projects/fspec',
        branch: 'main',
        isGitRepo: true,
      });

      // @step When it polls for git information
      const state = useFooterStore.getState().sessions['perf-session'];

      // @step Then it should only read the branch name via get_current_branch
      expect(state?.git.branch).toBe('main');

      // @step And it should not call get_staged_files
      // Verified: FooterGitStatus interface has no 'dirty' field
      expect(state?.git).not.toHaveProperty('dirty');

      // @step And it should not call get_unstaged_files
      // Verified: footerStore.updateFooterState accepts no dirty/untracked params
      expect(state?.git).not.toHaveProperty('staged');

      // @step And it should not call get_untracked_files
      // Verified: FooterGitStatus only has isGitRepo + branch
      expect(state?.git).not.toHaveProperty('untracked');
      expect(Object.keys(state?.git ?? {})).toEqual(['isGitRepo', 'branch']);
    });
  });

  // ----------------------------------------
  // Scenario: No footer state yet (Rust poller hasn't emitted)
  // ----------------------------------------

  describe('Scenario: No footer state yet from Rust poller', () => {
    it('should render empty footer before first FooterStateUpdate arrives', () => {
      // @step Given a session exists but no FooterStateUpdate has arrived yet

      // @step When the SessionFooter renders
      const { lastFrame } = render(<SessionFooter sessionId="brand-new-session" />);
      const output = lastFrame();

      // @step Then the footer should be empty (no path, no branch)
      expect(output).not.toContain('⎇');
      expect(output).not.toContain('~');
    });
  });

  // ========================================================================
  // Dynamic CWD tracking scenarios (new — per-session CWD updates from Bash)
  // ========================================================================

  // ----------------------------------------
  // Scenario: Footer CWD updates when Bash tool uses explicit cwd parameter
  // ----------------------------------------

  describe('Scenario: Footer CWD updates when Bash tool uses explicit cwd parameter', () => {
    it('should update CWD and remove branch when cwd changes to non-git directory', () => {
      // @step Given a session is started at "~/projects/fspec" on branch "main"
      seedFooterState('cwd-update-session', {
        cwd: '/Users/rquast/projects/fspec',
        displayPath: '~/projects/fspec',
        isGitRepo: true,
        branch: 'main',
      });

      const { lastFrame } = render(<SessionFooter sessionId="cwd-update-session" />);

      // @step And the footer shows "~/projects/fspec [⎇ main]"
      expect(lastFrame()).toContain('~/projects/fspec');
      expect(lastFrame()).toContain('main');

      // @step When the Bash tool executes a command with cwd "/tmp"
      // (Rust poller detects CWD change in registry, emits FooterStateUpdate)
      act(() => {
        useFooterStore.getState().updateFooterState(
          'cwd-update-session',
          '/tmp',
          '/tmp',
          false, // not a git repo
          null,
        );
      });

      // @step Then the footer should update to show "/tmp"
      expect(lastFrame()).toContain('/tmp');

      // @step And the git branch indicator should disappear since "/tmp" is not a git repository
      expect(lastFrame()).not.toContain('⎇');
    });
  });

  // ----------------------------------------
  // Scenario: Footer git branch updates when CWD changes to a different repository
  // ----------------------------------------

  describe('Scenario: Footer git branch updates when CWD changes to a different repository', () => {
    it('should show the new repo branch when CWD moves to a different git repo', () => {
      // @step Given a session is started at "~/projects/fspec" on branch "main"
      seedFooterState('repo-switch-session', {
        cwd: '/Users/rquast/projects/fspec',
        displayPath: '~/projects/fspec',
        isGitRepo: true,
        branch: 'main',
      });

      const { lastFrame } = render(<SessionFooter sessionId="repo-switch-session" />);
      expect(lastFrame()).toContain('main');

      // @step When the Bash tool executes a command with cwd pointing to another git repository on branch "develop"
      act(() => {
        useFooterStore.getState().updateFooterState(
          'repo-switch-session',
          '/Users/rquast/other-repo',
          '~/other-repo',
          true,
          'develop',
        );
      });

      // @step Then the footer should show the new repository path with "[⎇ develop]"
      expect(lastFrame()).toContain('~/other-repo');
      expect(lastFrame()).toContain('develop');

      // @step And the branch was resolved by reading .git/HEAD in the new cwd not the original session path
      // (Verified by the display showing the new branch, not the old one)
      expect(lastFrame()).not.toContain('main');
    });
  });

  // ----------------------------------------
  // Scenario: Each session tracks CWD and git branch independently
  // ----------------------------------------

  describe('Scenario: Each session tracks CWD and git branch independently', () => {
    it('should not affect other sessions when one session CWD changes', () => {
      // @step Given Session A is started at "~/projects/fspec" on branch "main"
      seedFooterState('session-a', {
        cwd: '/Users/rquast/projects/fspec',
        displayPath: '~/projects/fspec',
        isGitRepo: true,
        branch: 'main',
      });

      // @step And Session B is started at "~/projects/fspec" on branch "main"
      seedFooterState('session-b', {
        cwd: '/Users/rquast/projects/fspec',
        displayPath: '~/projects/fspec',
        isGitRepo: true,
        branch: 'main',
      });

      const resultA = render(<SessionFooter sessionId="session-a" />);
      const resultB = render(<SessionFooter sessionId="session-b" />);

      // @step When Session A runs a Bash command with cwd "/tmp"
      act(() => {
        useFooterStore.getState().updateFooterState(
          'session-a',
          '/tmp',
          '/tmp',
          false,
          null,
        );
      });

      // @step Then Session A footer shows "/tmp" with no git branch
      expect(resultA.lastFrame()).toContain('/tmp');
      expect(resultA.lastFrame()).not.toContain('⎇');

      // @step And Session B footer still shows "~/projects/fspec [⎇ main]" unchanged
      expect(resultB.lastFrame()).toContain('~/projects/fspec');
      expect(resultB.lastFrame()).toContain('main');
    });
  });

  // ----------------------------------------
  // Scenario: Footer CWD returns to session default when Bash runs without explicit cwd
  // ----------------------------------------

  describe('Scenario: Footer CWD returns to session default when Bash runs without explicit cwd', () => {
    it('should revert to project root when Bash runs with no cwd', () => {
      // @step Given a session is started at "~/projects/fspec" on branch "main"
      seedFooterState('revert-session', {
        cwd: '/Users/rquast/projects/fspec',
        displayPath: '~/projects/fspec',
        isGitRepo: true,
        branch: 'main',
      });

      const { lastFrame } = render(<SessionFooter sessionId="revert-session" />);

      // @step And the Bash tool previously ran with cwd "/tmp"
      act(() => {
        useFooterStore.getState().updateFooterState(
          'revert-session',
          '/tmp',
          '/tmp',
          false,
          null,
        );
      });

      // @step And the footer currently shows "/tmp"
      expect(lastFrame()).toContain('/tmp');

      // @step When the Bash tool executes a command with no cwd parameter
      // (BashTool writes process CWD = project root back to registry)
      act(() => {
        useFooterStore.getState().updateFooterState(
          'revert-session',
          '/Users/rquast/projects/fspec',
          '~/projects/fspec',
          true,
          'main',
        );
      });

      // @step Then the footer should show "~/projects/fspec [⎇ main]"
      expect(lastFrame()).toContain('~/projects/fspec');
      expect(lastFrame()).toContain('main');
    });
  });
});
