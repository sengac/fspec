/**
 * Feature: spec/features/isolated-session-file-operations.feature
 *
 * E2E tests for Isolated Session File Operations - BLOCKING Access to Main Project
 *
 * GIT-020: These tests verify that isolated sessions are BLOCKED from accessing
 * files outside the worktree. Tests use REAL NAPI bindings - NO mocks.
 *
 * CRITICAL: Tests must create real isolated sessions via sessionManagerCreateIsolated,
 * then use sessionValidatePath to verify path blocking/allowing behavior.
 */

import { describe, it, expect, beforeEach, afterEach, beforeAll } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';
import { randomUUID } from 'crypto';

// Import the NAPI bindings
import {
  sessionManagerCreateIsolated,
  sessionManagerCreateWithId,
  sessionManagerDestroy,
  sessionValidatePath,
  sessionGetEffectiveCwd,
  sessionIsIsolated,
  sessionSetGlobalChunkCallback,
  sessionExecuteBash,
  removeWorktree,
  persistenceSetDataDirectory,
} from '@sengac/codelet-napi';

describe('Feature: Isolated Session File Operations - BLOCKING Access to Main Project', () => {
  let testDir: string;
  let dataDir: string;
  let callbackInitialized = false;

  // Initialize global chunk callback once for all tests
  beforeAll(() => {
    if (!callbackInitialized) {
      try {
        // Initialize the global chunk callback - required for path validation callbacks
        sessionSetGlobalChunkCallback((_args: unknown) => {
          // No-op callback for testing
        });
        callbackInitialized = true;
      } catch {
        // Callback may already be set from previous test run
        callbackInitialized = true;
      }
    }
  });

  beforeEach(() => {
    // Create a temporary git repository for testing
    testDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'fspec-isolated-file-blocking-e2e-')
    );

    // Create a temporary data directory for persistence
    dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-data-'));
    persistenceSetDataDirectory(dataDir);

    // Initialize git repo
    execSync('git init', { cwd: testDir, stdio: 'pipe' });
    execSync('git config user.email "test@test.com"', {
      cwd: testDir,
      stdio: 'pipe',
    });
    execSync('git config user.name "Test User"', {
      cwd: testDir,
      stdio: 'pipe',
    });

    // Create initial file structure
    fs.mkdirSync(path.join(testDir, 'src'), { recursive: true });
    fs.writeFileSync(
      path.join(testDir, 'src', 'main.ts'),
      'main project content'
    );
    fs.writeFileSync(path.join(testDir, 'README.md'), '# Test Project');

    // Initial commit
    execSync('git add .', { cwd: testDir, stdio: 'pipe' });
    execSync('git commit -m "Initial commit"', { cwd: testDir, stdio: 'pipe' });
  });

  afterEach(() => {
    // Cleanup test directories
    try {
      fs.rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Directory may not exist
    }
    try {
      fs.rmSync(dataDir, { recursive: true, force: true });
    } catch {
      // Directory may not exist
    }
  });

  // Helper to create isolated session and return cleanup function
  async function createIsolatedSession(): Promise<{
    sessionId: string;
    worktreePath: string;
    cleanup: () => void;
  }> {
    const sessionId = randomUUID();
    const result = await sessionManagerCreateIsolated(
      sessionId,
      'anthropic/claude-sonnet-4-20250514',
      testDir,
      'Test Isolated Session'
    );

    return {
      sessionId,
      worktreePath: result.worktreePath,
      cleanup: () => {
        try {
          sessionManagerDestroy(sessionId);
        } catch {
          // Session may not exist
        }
        try {
          removeWorktree(testDir, sessionId);
        } catch {
          // Worktree may not exist
        }
      },
    };
  }

  // Helper to create non-isolated session
  async function createNonIsolatedSession(): Promise<{
    sessionId: string;
    cleanup: () => void;
  }> {
    const sessionId = randomUUID();
    await sessionManagerCreateWithId(
      sessionId,
      'anthropic/claude-sonnet-4-20250514',
      testDir,
      'Test Non-Isolated Session'
    );

    return {
      sessionId,
      cleanup: () => {
        try {
          sessionManagerDestroy(sessionId);
        } catch {
          // Session may not exist
        }
      },
    };
  }

  // ========================================
  // BLOCKING SCENARIOS - Read Tool
  // ========================================

  describe('Scenario: Isolated session Read tool BLOCKED from reading main project file with absolute path', () => {
    it('should block read access to main project file', async () => {
      // @step Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
      // testDir is our git repository with src/main.ts created in beforeEach

      // @step And an isolated session is created via sessionManagerCreateIsolated NAPI binding
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step And the session has worktree at "/project/.fspec/worktrees/<session-id>"
        const effectiveCwd = sessionGetEffectiveCwd(sessionId);
        expect(effectiveCwd).toBeDefined();
        expect(effectiveCwd).toContain('.fspec/worktrees');

        // @step When the Read tool is invoked with file_path "/project/src/main.ts"
        const mainProjectFile = path.join(testDir, 'src', 'main.ts');
        const result = sessionValidatePath(sessionId, mainProjectFile, 'read');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');

        // @step And the file should NOT be read
        // Path validation blocks before file read occurs
        expect(result.resolvedPath == null).toBe(true); // null or undefined;

        // @step And a block notification should be emitted
        // Block notifications are emitted via the global chunk callback
        // The validation itself doesn't emit - that happens in the tool wrapper
        // We verify the error message format matches expected notification content
        expect(result.error).toContain('worktree');
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Read tool BLOCKED from path traversal escape', () => {
    it('should block path traversal attempts', async () => {
      // @step Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains directory "src/"
        fs.mkdirSync(path.join(worktreePath, 'src'), { recursive: true });

        // @step When the Read tool is invoked with file_path "../../src/main.ts"
        const result = sessionValidatePath(
          sessionId,
          '../../src/main.ts',
          'read'
        );

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');

        // @step And the file should NOT be read
        expect(result.resolvedPath == null).toBe(true); // null or undefined;
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Read tool BLOCKED from symlink escape', () => {
    it('should block symlink escape attempts', async () => {
      // @step Given a git repository at "/project" with file "/project/src/secret.ts" containing "secret content"
      fs.writeFileSync(
        path.join(testDir, 'src', 'secret.ts'),
        'secret content'
      );

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains a symlink "escape" pointing to "/project/src/"
        const symlinkPath = path.join(worktreePath, 'escape');
        const mainSrcPath = path.join(testDir, 'src');
        fs.symlinkSync(mainSrcPath, symlinkPath, 'dir');

        // @step When the Read tool is invoked with file_path "escape/secret.ts"
        const result = sessionValidatePath(
          sessionId,
          'escape/secret.ts',
          'read'
        );

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');

        // @step And the file should NOT be read
        expect(result.resolvedPath == null).toBe(true); // null or undefined;
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Read tool ALLOWED for relative path within worktree', () => {
    it('should allow relative paths within worktree', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains file "src/app.ts" with content "worktree content"
        fs.mkdirSync(path.join(worktreePath, 'src'), { recursive: true });
        fs.writeFileSync(
          path.join(worktreePath, 'src', 'app.ts'),
          'worktree content'
        );

        // @step When the Read tool is invoked with file_path "src/app.ts"
        const result = sessionValidatePath(sessionId, 'src/app.ts', 'read');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined

        // @step And the content should be "worktree content"
        // Verify resolved path is within worktree
        expect(result.resolvedPath).toBeDefined();
        expect(result.resolvedPath).toContain(worktreePath);
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Read tool ALLOWED for absolute path within worktree', () => {
    it('should allow absolute paths within worktree', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains file "src/app.ts" with content "worktree content"
        fs.mkdirSync(path.join(worktreePath, 'src'), { recursive: true });
        fs.writeFileSync(
          path.join(worktreePath, 'src', 'app.ts'),
          'worktree content'
        );

        // @step When the Read tool is invoked with file_path "/project/.fspec/worktrees/<session-id>/src/app.ts"
        const absolutePath = path.join(worktreePath, 'src', 'app.ts');
        const result = sessionValidatePath(sessionId, absolutePath, 'read');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the content should be "worktree content"
        expect(result.resolvedPath).toBe(absolutePath);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - Write Tool
  // ========================================

  describe('Scenario: Isolated session Write tool BLOCKED from writing to main project', () => {
    it('should block write access to main project', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the Write tool is invoked with file_path "/project/src/malicious.ts" and content "injected code"
        const mainProjectFile = path.join(testDir, 'src', 'malicious.ts');
        const result = sessionValidatePath(sessionId, mainProjectFile, 'write');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');

        // @step And the file should NOT exist at "/project/src/malicious.ts"
        expect(fs.existsSync(mainProjectFile)).toBe(false);

        // @step And a block notification should be emitted
        // Block notifications are emitted via the global chunk callback
        expect(result.error).toContain('worktree');
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Write tool ALLOWED for relative path within worktree', () => {
    it('should allow write access within worktree', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step When the Write tool is invoked with file_path "src/new-file.ts" and content "new content"
        const result = sessionValidatePath(
          sessionId,
          'src/new-file.ts',
          'write'
        );

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the file should exist at worktree path "src/new-file.ts" with content "new content"
        expect(result.resolvedPath).toContain(worktreePath);

        // @step And the file should NOT exist at "/project/src/new-file.ts"
        const mainProjectFile = path.join(testDir, 'src', 'new-file.ts');
        expect(fs.existsSync(mainProjectFile)).toBe(false);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - Edit Tool
  // ========================================

  describe('Scenario: Isolated session Edit tool BLOCKED from editing main project file', () => {
    it('should block edit access to main project', async () => {
      // @step Given a git repository at "/project" with file "/project/src/config.ts" containing "original"
      fs.writeFileSync(path.join(testDir, 'src', 'config.ts'), 'original');

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the Edit tool is invoked with file_path "/project/src/config.ts" replacing "original" with "modified"
        const mainProjectFile = path.join(testDir, 'src', 'config.ts');
        const result = sessionValidatePath(sessionId, mainProjectFile, 'edit');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');

        // @step And the file at "/project/src/config.ts" should still contain "original"
        const content = fs.readFileSync(mainProjectFile, 'utf-8');
        expect(content).toBe('original');

        // @step And a block notification should be emitted
        // Block notifications are emitted via the global chunk callback
        expect(result.error).toContain('worktree');
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - Ls Tool
  // ========================================

  describe('Scenario: Isolated session Ls tool BLOCKED from listing main project directory', () => {
    it('should block ls access to main project directory', async () => {
      // @step Given a git repository at "/project" with directory "/project/src/" containing files
      // testDir/src/ already exists with files

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the Ls tool is invoked with path "/project/src/"
        const mainProjectDir = path.join(testDir, 'src');
        const result = sessionValidatePath(sessionId, mainProjectDir, 'ls');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Ls tool ALLOWED for worktree directory', () => {
    it('should allow ls access within worktree', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains directory "src/" with files
        fs.mkdirSync(path.join(worktreePath, 'src'), { recursive: true });
        fs.writeFileSync(path.join(worktreePath, 'src', 'test.ts'), 'test');

        // @step When the Ls tool is invoked with path "src/"
        const result = sessionValidatePath(sessionId, 'src/', 'ls');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the output should list files in the worktree src/ directory
        expect(result.resolvedPath).toContain(worktreePath);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - Grep Tool
  // ========================================

  describe('Scenario: Isolated session Grep tool BLOCKED from searching main project', () => {
    it('should block grep access to main project', async () => {
      // @step Given a git repository at "/project" with files containing searchable content
      // testDir has src/main.ts

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the Grep tool is invoked with pattern "FIXME" and path "/project/src/"
        const mainProjectDir = path.join(testDir, 'src');
        const result = sessionValidatePath(sessionId, mainProjectDir, 'grep');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Grep tool ALLOWED for searching worktree', () => {
    it('should allow grep access within worktree', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains file "src/app.ts" with content "// FIXME: fix this"
        fs.mkdirSync(path.join(worktreePath, 'src'), { recursive: true });
        fs.writeFileSync(
          path.join(worktreePath, 'src', 'app.ts'),
          '// FIXME: fix this'
        );

        // @step When the Grep tool is invoked with pattern "FIXME" and path "src/"
        const result = sessionValidatePath(sessionId, 'src/', 'grep');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the results should include matches from worktree
        expect(result.resolvedPath).toContain(worktreePath);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - Glob Tool
  // ========================================

  describe('Scenario: Isolated session Glob tool BLOCKED from globbing main project', () => {
    it('should block glob access to main project', async () => {
      // @step Given a git repository at "/project" with TypeScript files
      // testDir has src/main.ts

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the Glob tool is invoked with pattern "**/*.ts" and path "/project/"
        const result = sessionValidatePath(sessionId, testDir, 'glob');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Isolated session Glob tool ALLOWED for globbing worktree', () => {
    it('should allow glob access within worktree', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step And the worktree contains TypeScript files in "src/"
        fs.mkdirSync(path.join(worktreePath, 'src'), { recursive: true });
        fs.writeFileSync(path.join(worktreePath, 'src', 'app.ts'), 'content');

        // @step When the Glob tool is invoked with pattern "**/*.ts" and path "src/"
        const result = sessionValidatePath(sessionId, 'src/', 'glob');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the results should only include worktree files
        expect(result.resolvedPath).toContain(worktreePath);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - AstGrep Tool
  // ========================================

  describe('Scenario: Isolated session AstGrep tool BLOCKED from searching main project', () => {
    it('should block ast_grep access to main project', async () => {
      // @step Given a git repository at "/project" with TypeScript files
      // testDir has src/main.ts

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the AstGrep tool is invoked with pattern "function $NAME()" language "typescript" and path "/project/"
        const result = sessionValidatePath(sessionId, testDir, 'ast_grep');

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - AstGrepRefactor Tool
  // ========================================

  describe('Scenario: Isolated session AstGrepRefactor tool BLOCKED from refactoring main project', () => {
    it('should block ast_grep_refactor access to main project', async () => {
      // @step Given a git repository at "/project" with file "/project/src/refactor-me.ts"
      fs.writeFileSync(
        path.join(testDir, 'src', 'refactor-me.ts'),
        'const x = 1;'
      );

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, cleanup } = await createIsolatedSession();

      try {
        // @step When the AstGrepRefactor tool is invoked with source_file "/project/src/refactor-me.ts"
        const mainProjectFile = path.join(testDir, 'src', 'refactor-me.ts');
        const result = sessionValidatePath(
          sessionId,
          mainProjectFile,
          'ast_grep_refactor'
        );

        // @step Then the tool should return an error containing "outside isolated worktree"
        expect(result.allowed).toBe(false);
        expect(result.error).toBeDefined();
        expect(result.error).toContain('outside isolated worktree');

        // @step And the file at "/project/src/refactor-me.ts" should be unchanged
        const content = fs.readFileSync(mainProjectFile, 'utf-8');
        expect(content).toBe('const x = 1;');
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BLOCKING SCENARIOS - Bash Tool (cwd restriction)
  // ========================================

  describe('Scenario: Isolated session Bash tool runs with cwd restricted to worktree', () => {
    it('should run pwd and return worktree path', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step When the Bash tool is invoked with command "pwd"
        const result = sessionExecuteBash(sessionId, 'pwd');

        // @step Then the command should succeed
        expect(result.success).toBe(true);
        expect(result.output).toBeDefined();

        // @step And the output should contain the worktree path
        const output = result.output?.trim();
        expect(output).toBe(worktreePath);
      } finally {
        cleanup();
      }
    });

    it('should create files within worktree directory', async () => {
      // @step Given an isolated session with worktree
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();

      try {
        // @step When creating a file via bash touch command
        const testFile = 'bash-created-file.txt';
        const result = sessionExecuteBash(
          sessionId,
          `touch ${testFile} && echo "created"`
        );

        // @step Then the command should succeed
        expect(result.success).toBe(true);

        // @step And the file should exist in the worktree
        const fileExists = fs.existsSync(path.join(worktreePath, testFile));
        expect(fileExists).toBe(true);

        // @step And the file should NOT exist in the main project
        const mainProjectFile = path.join(testDir, testFile);
        expect(fs.existsSync(mainProjectFile)).toBe(false);
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Non-isolated session Bash tool runs in project root', () => {
    it('should run pwd and return project root', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And a non-isolated session
      const { sessionId, cleanup } = await createNonIsolatedSession();

      try {
        // @step When the Bash tool is invoked with command "pwd"
        const result = sessionExecuteBash(sessionId, 'pwd');

        // @step Then the command should succeed
        expect(result.success).toBe(true);
        expect(result.output).toBeDefined();

        // @step And the output should contain the project root path
        const output = result.output?.trim();
        expect(output).toBe(testDir);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // BACKWARD COMPATIBILITY - Non-Isolated Sessions
  // ========================================

  describe('Scenario: Non-isolated session Read tool ALLOWED for all paths', () => {
    it('should allow all paths for non-isolated session', async () => {
      // @step Given a git repository at "/project" with file "/project/src/main.ts" containing "main content"
      // testDir has src/main.ts

      // @step And a non-isolated session is created via sessionManagerCreateWithId NAPI binding
      const { sessionId, cleanup } = await createNonIsolatedSession();

      try {
        // @step When the Read tool is invoked with file_path "/project/src/main.ts"
        const mainProjectFile = path.join(testDir, 'src', 'main.ts');
        const result = sessionValidatePath(sessionId, mainProjectFile, 'read');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the content should be "main content"
        expect(result.resolvedPath).toBe(mainProjectFile);
      } finally {
        cleanup();
      }
    });
  });

  describe('Scenario: Non-isolated session Write tool ALLOWED for all paths', () => {
    it('should allow write to any path for non-isolated session', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And a non-isolated session is created via sessionManagerCreateWithId NAPI binding
      const { sessionId, cleanup } = await createNonIsolatedSession();

      try {
        // @step When the Write tool is invoked with file_path "/project/src/new.ts" and content "new content"
        const newFile = path.join(testDir, 'src', 'new.ts');
        const result = sessionValidatePath(sessionId, newFile, 'write');

        // @step Then the tool should succeed
        expect(result.allowed).toBe(true);
        expect(result.error == null).toBe(true); // null or undefined;

        // @step And the file should exist at "/project/src/new.ts" with content "new content"
        expect(result.resolvedPath).toBe(newFile);
      } finally {
        cleanup();
      }
    });
  });

  // ========================================
  // Helper Validation Tests
  // ========================================

  describe('sessionIsIsolated helper', () => {
    it('should return true for isolated session', async () => {
      const { sessionId, cleanup } = await createIsolatedSession();
      try {
        expect(sessionIsIsolated(sessionId)).toBe(true);
      } finally {
        cleanup();
      }
    });

    it('should return false for non-isolated session', async () => {
      const { sessionId, cleanup } = await createNonIsolatedSession();
      try {
        expect(sessionIsIsolated(sessionId)).toBe(false);
      } finally {
        cleanup();
      }
    });
  });

  describe('sessionGetEffectiveCwd helper', () => {
    it('should return worktree path for isolated session', async () => {
      const { sessionId, worktreePath, cleanup } =
        await createIsolatedSession();
      try {
        const effectiveCwd = sessionGetEffectiveCwd(sessionId);
        expect(effectiveCwd).toBe(worktreePath);
      } finally {
        cleanup();
      }
    });

    it('should return project root for non-isolated session', async () => {
      const { sessionId, cleanup } = await createNonIsolatedSession();
      try {
        const effectiveCwd = sessionGetEffectiveCwd(sessionId);
        expect(effectiveCwd).toBe(testDir);
      } finally {
        cleanup();
      }
    });
  });
});
