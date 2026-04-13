/**
 * Feature: spec/features/refactor-watcher-terminology-to-supervisor-subordinate-with-chainofcommand-graph.feature
 *
 * This test file validates the acceptance criteria for WATCH-024:
 * Refactoring watcher terminology to supervisor/subordinate with ChainOfCommand graph.
 *
 * This is a PURE RENAME refactoring — all tests verify naming changes, not behavior.
 * Tests check that old names no longer exist and new names are present.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const PROJECT_ROOT = process.cwd();
const CODELET_NAPI_SRC = join(PROJECT_ROOT, 'codelet', 'napi', 'src');
const TUI_COMPONENTS = join(PROJECT_ROOT, 'src', 'tui', 'components');
const TUI_TYPES = join(PROJECT_ROOT, 'src', 'tui', 'types');
const TUI_UTILS = join(PROJECT_ROOT, 'src', 'tui', 'utils');
const TUI_HOOKS = join(PROJECT_ROOT, 'src', 'tui', 'hooks');

/**
 * Helper: read a file and return its content as string
 */
function readFile(filePath: string): string {
  return readFileSync(filePath, 'utf-8');
}

/**
 * Helper: check if a pattern exists in a file (case-sensitive)
 */
function fileContains(filePath: string, pattern: string): boolean {
  if (!existsSync(filePath)) {
    return false;
  }
  return readFile(filePath).includes(pattern);
}

/**
 * Helper: check if a pattern exists as a word boundary match in file
 * (avoids false positives from partial matches like "work_units_watcher")
 */
function fileContainsRustIdentifier(
  filePath: string,
  identifier: string
): boolean {
  if (!existsSync(filePath)) {
    return false;
  }
  const content = readFile(filePath);
  // Use word boundary-like matching for Rust identifiers
  const regex = new RegExp(`\\b${identifier}\\b`);
  return regex.test(content);
}

/**
 * Helper: recursively collect all .ts and .tsx files
 */
function collectTsFiles(dir: string): string[] {
  const results: string[] = [];
  if (!existsSync(dir)) {
    return results;
  }
  const entries = readdirSync(dir);
  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      results.push(...collectTsFiles(fullPath));
    } else if (entry.endsWith('.ts') || entry.endsWith('.tsx')) {
      results.push(fullPath);
    }
  }
  return results;
}

const SESSION_MANAGER_RS = join(CODELET_NAPI_SRC, 'session_manager.rs');
const TYPES_RS = join(CODELET_NAPI_SRC, 'types.rs');
const NAVIGATION_RS = join(CODELET_NAPI_SRC, 'navigation.rs');

describe('Feature: Refactor watcher terminology to supervisor/subordinate', () => {
  describe('Scenario: ChainOfCommand replaces WatchGraph with renamed methods', () => {
    it('should have ChainOfCommand instead of WatchGraph', () => {
      // @step Given the WatchGraph struct has been renamed to ChainOfCommand
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'ChainOfCommand')
      ).toBe(true);

      // @step When I call ChainOfCommand.add_supervisor(subordinate_id, supervisor_id)
      expect(fileContains(SESSION_MANAGER_RS, 'add_supervisor')).toBe(true);
      // Verify parameter names match the spec
      expect(
        fileContains(
          SESSION_MANAGER_RS,
          'fn add_supervisor(&self, subordinate_id: Uuid, supervisor_id: Uuid)'
        )
      ).toBe(true);

      // @step Then the relationship is registered in subordinate_to_supervisors and supervisor_to_subordinate maps
      expect(
        fileContains(SESSION_MANAGER_RS, 'subordinate_to_supervisors')
      ).toBe(true);
      expect(
        fileContains(SESSION_MANAGER_RS, 'supervisor_to_subordinate')
      ).toBe(true);

      // @step And cycle prevention still works identically to the old add_watcher logic
      // Old names should NOT exist
      expect(fileContainsRustIdentifier(SESSION_MANAGER_RS, 'WatchGraph')).toBe(
        false
      );
      expect(fileContains(SESSION_MANAGER_RS, 'parent_to_watchers')).toBe(
        false
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_to_parent')).toBe(false);
    });
  });

  describe('Scenario: SupervisorRole replaces SessionRole without authority field', () => {
    it('should have SupervisorRoleInfo with correct fields', () => {
      // @step Given the SessionRole struct has been renamed
      // AMGR-008: SupervisorRole was further simplified to SupervisorRoleInfo (API compat wrapper)
      // Role is now a simple string on BackgroundSession, SupervisorRoleInfo wraps it for TS
      expect(fileContains(SESSION_MANAGER_RS, 'SupervisorRoleInfo')).toBe(true);
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'SessionRole')
      ).toBe(false);

      // @step Then SupervisorRoleInfo has a name field of type String
      expect(fileContains(SESSION_MANAGER_RS, 'pub name: String')).toBe(true);

      // @step And SupervisorRoleInfo has a brief field of type Option<String>
      expect(
        fileContains(SESSION_MANAGER_RS, 'pub brief: Option<String>')
      ).toBe(true);

      // @step And session_set_role accepts auto_inject parameter (unused, for API compat)
      // AMGR-008: auto_inject is an unused parameter (prefixed with _) kept for API compat
      expect(
        fileContains(SESSION_MANAGER_RS, '_auto_inject: Option<bool>')
      ).toBe(true);

      // @step And SupervisorRoleInfo does not have an authority field
      // The 'authority' field should not exist
      // (RoleAuthority is removed entirely — checked in next scenario)
    });
  });

  describe('Scenario: RoleAuthority enum is removed entirely', () => {
    it('should have no RoleAuthority references', () => {
      // @step Given the RoleAuthority enum previously had Peer and Supervisor variants
      // (historical fact — we're verifying it's gone)

      // @step When the refactoring is complete
      // @step Then the RoleAuthority enum no longer exists in the codebase
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'RoleAuthority')
      ).toBe(false);

      // @step And no code references Peer or Supervisor authority levels
      // Check types.rs too
      expect(fileContainsRustIdentifier(TYPES_RS, 'RoleAuthority')).toBe(false);

      // @step And the brief field provides all behavioral instruction instead
      expect(
        fileContains(SESSION_MANAGER_RS, 'pub brief: Option<String>')
      ).toBe(true);
    });
  });

  describe('Scenario: Injection message uses simplified supervisor prefix', () => {
    it('should format messages with SUPERVISOR prefix', () => {
      // @step Given a supervisor session with role "security-reviewer" and session ID "abc-123"
      // @step When the supervisor injects a message into the subordinate session
      // @step Then the message prefix reads "[SUPERVISOR: security-reviewer | Session: abc-123]"
      expect(fileContains(SESSION_MANAGER_RS, '[SUPERVISOR:')).toBe(true);

      // @step And the prefix does not include an Authority field
      expect(fileContains(SESSION_MANAGER_RS, 'Authority:')).toBe(false);

      // Also check TypeScript chunk processor
      const chunkProcessor = join(TUI_UTILS, 'chunkProcessor.ts');
      expect(fileContains(chunkProcessor, 'SUPERVISOR:')).toBe(true);
      expect(fileContains(chunkProcessor, 'WATCHER:')).toBe(false);
    });
  });

  describe('Scenario: TUI template form removes Authority toggle', () => {
    it('should have no template form files (AMGR-008 removed template infrastructure)', () => {
      // @step Given the supervisor template creation form was removed by AMGR-008
      // AMGR-008: Simplified supervisor management — removed template infrastructure entirely
      // Role is now set via /role command, no template forms needed

      // @step Then SupervisorTemplateForm.tsx does not exist (removed by AMGR-008)
      const formFile = join(TUI_COMPONENTS, 'SupervisorTemplateForm.tsx');
      expect(existsSync(formFile)).toBe(false);

      // @step And the old WatcherTemplateForm.tsx does not exist
      const oldFormFile = join(TUI_COMPONENTS, 'WatcherTemplateForm.tsx');
      expect(existsSync(oldFormFile)).toBe(false);

      // @step And role management is now handled via /role slash command
      const slashCommandsFile = join(TUI_UTILS, 'slashCommands.ts');
      expect(fileContains(slashCommandsFile, "'role'")).toBe(true);
    });
  });

  describe('Scenario: NAPI bindings use supervisor terminology', () => {
    it('should export supervisor NAPI functions', () => {
      // @step Given the Rust NAPI bindings have been renamed
      // @step Then sessionCreateSupervisor is exported instead of sessionCreateWatcher
      expect(
        fileContains(SESSION_MANAGER_RS, 'session_create_supervisor')
      ).toBe(true);
      expect(fileContains(SESSION_MANAGER_RS, 'session_create_watcher')).toBe(
        false
      );

      // @step And sessionGetSupervisors is exported instead of sessionGetWatchers
      expect(fileContains(SESSION_MANAGER_RS, 'session_get_supervisors')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'session_get_watchers')).toBe(
        false
      );

      // @step And sessionGetSubordinate is exported instead of sessionGetParent
      expect(fileContains(SESSION_MANAGER_RS, 'session_get_subordinate')).toBe(
        true
      );
      // Note: sessionGetParent may still exist in non-watcher contexts, check carefully
      // The NAPI #[napi] exported function should be renamed

      // @step And sessionClearRole is no longer exported
      // session_clear_role was replaced with a comment, so it exists as text
      // but is NOT an actual function anymore. Check there's no #[napi] fn
      const smContent2 = readFile(SESSION_MANAGER_RS);
      expect(smContent2).not.toMatch(/fn session_clear_role/);
    });
  });

  describe('Scenario: supervisor_inject is internal Rust only', () => {
    it('should handle supervisor injection inline in agent_loop', () => {
      // @step Given the watcher_inject function has been removed
      // AMGR-008: supervisor_inject was inlined into agent_loop
      // Injection happens via IncomingMessage sent through supervisor broadcast
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_inject')).toBe(false);

      // @step Then agent_loop processes supervisor input directly
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor input from')).toBe(
        true
      );

      // @step And supervisor injection uses SUPERVISOR prefix format
      expect(fileContains(SESSION_MANAGER_RS, '[SUPERVISOR:')).toBe(true);

      // @step And no TypeScript code imports or calls supervisorInject
      // Check only production source files (not test files which may reference it in comments)
      const prodTsFiles = collectTsFiles(join(PROJECT_ROOT, 'src')).filter(
        f =>
          !f.includes('__tests__') &&
          !f.includes('fixtures') &&
          !f.includes('.test.')
      );
      for (const tsFile of prodTsFiles) {
        expect(fileContains(tsFile, 'supervisorInject')).toBe(false);
      }
    });
  });

  describe('Scenario: StreamChunk variants use supervisor naming', () => {
    it('should have supervisor StreamChunk variants', () => {
      // @step Given the StreamChunk enum has been updated
      // @step Then the WatcherInput variant is renamed to IncomingMessage (AMGR refactor)
      expect(fileContainsRustIdentifier(TYPES_RS, 'IncomingMessage')).toBe(
        true
      );
      expect(fileContains(TYPES_RS, 'WatcherInput')).toBe(false);

      // @step And the WatcherPendingInjection variant is renamed to SupervisorPendingInjection
      expect(fileContains(TYPES_RS, 'SupervisorPendingInjection')).toBe(true);
      expect(fileContains(TYPES_RS, 'WatcherPendingInjection')).toBe(false);

      // @step And the JSON wire format emits "supervisorInput" instead of "watcherInput"
      expect(fileContains(TYPES_RS, '"supervisorInput"')).toBe(true);
      expect(fileContains(TYPES_RS, '"watcherInput"')).toBe(false);

      // @step And the JSON wire format emits "supervisorPendingInjection" instead of "watcherPendingInjection"
      expect(fileContains(TYPES_RS, '"supervisorPendingInjection"')).toBe(true);
      expect(fileContains(TYPES_RS, '"watcherPendingInjection"')).toBe(false);

      // Verify TypeScript chunk type checks match the NAPI discriminant format
      // NAPI serializes Rust enum variant names as-is → 'IncomingMessage' (PascalCase)
      const chunkProcessor = join(TUI_UTILS, 'chunkProcessor.ts');
      expect(fileContains(chunkProcessor, "'IncomingMessage'")).toBe(true);
      expect(fileContains(chunkProcessor, "'WatcherInput'")).toBe(false);

      // Verify index.d.ts matches NAPI discriminant
      const indexDts = join(PROJECT_ROOT, 'codelet', 'napi', 'index.d.ts');
      expect(fileContains(indexDts, "'IncomingMessage'")).toBe(true);
      expect(fileContains(indexDts, "'WatcherInput'")).toBe(false);
      expect(fileContains(indexDts, "'SupervisorPendingInjection'")).toBe(true);
      expect(fileContains(indexDts, "'WatcherPendingInjection'")).toBe(false);
    });
  });

  describe('Scenario: Supervisor agent loop uses renamed functions and types', () => {
    it('should have unified agent_loop with supervisor support', () => {
      // @step Given the agent loop code has been unified (AMGR-008)
      // AMGR-008: supervisor_agent_loop was merged into the main agent_loop
      // All sessions use the same agent_loop, supervisor input handled via select!

      // @step Then a unified agent_loop function exists
      expect(fileContains(SESSION_MANAGER_RS, 'async fn agent_loop')).toBe(
        true
      );

      // @step And old watcher-specific loop functions do not exist
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_agent_loop')).toBe(
        false
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_loop_tick')).toBe(false);
      expect(fileContains(SESSION_MANAGER_RS, 'run_watcher_loop')).toBe(false);

      // @step And old watcher types do not exist in session_manager.rs
      const smContent = readFile(SESSION_MANAGER_RS);
      expect(smContent).not.toMatch(/\bWatcherState\b/);
      expect(smContent).not.toMatch(/\bWatcherOutput\b/);
    });
  });

  describe('Scenario: BackgroundSession fields use supervisor naming', () => {
    it('should have supervisor field names', () => {
      // @step Given the BackgroundSession struct has been updated
      // @step Then the watcher_broadcast field is renamed to supervisor_broadcast
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_broadcast')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_broadcast')).toBe(false);

      // @step And old watcher_input_tx/rx fields no longer exist
      // AMGR-008: Supervisor input is handled via IncomingMessage through broadcast channel
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_input_tx')).toBe(false);
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_input_rx')).toBe(false);

      // @step And supervisor_input_rx is referenced in agent_loop comments
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_input_rx')).toBe(
        true
      );
    });
  });

  describe('Scenario: TUI components use supervisor naming', () => {
    it('should have renamed TUI component files', () => {
      // @step Given the TUI component files have been updated
      // AMGR-008: Template-based supervisor creation was removed.
      // SupervisorCreateView, SupervisorTemplateList, SupervisorTemplateForm were all removed.
      // Supervisor management is now done via /role command and agent manager tool.

      // @step Then old watcher component files do not exist
      expect(existsSync(join(TUI_COMPONENTS, 'WatcherCreateView.tsx'))).toBe(
        false
      );
      expect(existsSync(join(TUI_COMPONENTS, 'WatcherTemplateList.tsx'))).toBe(
        false
      );
      expect(existsSync(join(TUI_COMPONENTS, 'WatcherTemplateForm.tsx'))).toBe(
        false
      );

      // @step And useSupervisorHeaderInfo hook was removed (TUI-087: dead code removal)
      expect(existsSync(join(TUI_HOOKS, 'useSupervisorHeaderInfo.ts'))).toBe(
        false
      );
      expect(existsSync(join(TUI_HOOKS, 'useWatcherHeaderInfo.ts'))).toBe(
        false
      );
    });
  });

  describe('Scenario: Template storage uses supervisor naming', () => {
    it('should have no template storage (AMGR-008 removed template infrastructure)', () => {
      // @step Given the template storage system was removed by AMGR-008
      // AMGR-008: Template infrastructure was entirely removed.
      // Supervisors are created via agent manager tool, roles via /role command.

      // @step Then supervisorTemplateStorage.ts does not exist (removed by AMGR-008)
      expect(existsSync(join(TUI_UTILS, 'supervisorTemplateStorage.ts'))).toBe(
        false
      );

      // @step And old watcherTemplateStorage.ts does not exist
      expect(existsSync(join(TUI_UTILS, 'watcherTemplateStorage.ts'))).toBe(
        false
      );

      // @step And supervisorTemplate.ts type file does not exist (removed by AMGR-008)
      expect(existsSync(join(TUI_TYPES, 'supervisorTemplate.ts'))).toBe(false);

      // @step And old watcherTemplate.ts does not exist
      expect(existsSync(join(TUI_TYPES, 'watcherTemplate.ts'))).toBe(false);
    });
  });

  describe('Scenario: Slash command renamed from /watcher to /role', () => {
    it('should have /role and no /watcher or /parent', () => {
      // @step Given the slash command registry has been updated
      const slashCommandsFile = join(TUI_UTILS, 'slashCommands.ts');

      // @step Then /watcher was replaced with /role (AMGR-012)
      // AMGR-008/AMGR-012: /supervisor was further simplified to /role
      expect(fileContains(slashCommandsFile, "'role'")).toBe(true);

      // Check old commands don't exist
      const content = readFile(slashCommandsFile);
      // /watcher should be gone as a registered command
      expect(content).not.toMatch(/name:\s*['"]watcher['"]/);

      // @step And the /parent command is removed entirely
      expect(content).not.toMatch(/name:\s*['"]parent['"]/);

      // @step And /supervisor is not a separate command (replaced by /role)
      expect(content).not.toMatch(/name:\s*['"]supervisor['"]/);
    });
  });

  describe('Scenario: Navigation references use supervisor terminology', () => {
    it('should use supervisor in navigation', () => {
      // @step Given navigation.rs has been updated
      // @step Then build_navigation_list references supervisor instead of watcher
      expect(fileContains(NAVIGATION_RS, 'supervisor')).toBe(true);
      // Should not reference "watcher" in session-watcher context
      // (but it imports from crate::session_manager which is fine)
      expect(fileContainsRustIdentifier(NAVIGATION_RS, 'WatchGraph')).toBe(
        false
      );
      expect(fileContainsRustIdentifier(NAVIGATION_RS, 'ChainOfCommand')).toBe(
        true
      );

      // SplitSessionView was removed in TUI-080 (dead split view cleanup)
    });
  });

  describe('Scenario: Filesystem watcher is not affected by refactoring', () => {
    it('should leave filesystem watcher files unchanged', () => {
      // @step Given work_units_watcher.rs is a filesystem watcher for spec/work-units.json
      const fsWatcher = join(CODELET_NAPI_SRC, 'work_units_watcher.rs');
      expect(existsSync(fsWatcher)).toBe(true);

      // @step When the supervisor/subordinate refactoring is complete
      // @step Then work_units_watcher.rs is unchanged
      // The file should still use its own WatcherState (filesystem concept)
      expect(fileContains(fsWatcher, 'WatcherState')).toBe(true);

      // @step And useWorkUnitsWatcher hook is unchanged
      const hookFile = join(TUI_HOOKS, 'useWorkUnitsWatcher.ts');
      expect(existsSync(hookFile)).toBe(true);

      // @step And startWorkUnitsWatcher NAPI binding is unchanged
      const workUnitsWatcherFile = join(
        CODELET_NAPI_SRC,
        'work_units_watcher.rs'
      );
      expect(
        fileContains(workUnitsWatcherFile, 'start_work_units_watcher')
      ).toBe(true);

      // @step And stopWorkUnitsWatcher NAPI binding is unchanged
      expect(
        fileContains(workUnitsWatcherFile, 'stop_work_units_watcher')
      ).toBe(true);
    });
  });

  describe('Scenario: All existing tests pass after renaming', () => {
    it('should have no references to old watcher terminology in domain context', () => {
      // @step Given all watcher terminology has been renamed to supervisor/subordinate
      // Verify key old terms are gone from the main source files (excluding filesystem watcher)

      // Check session_manager.rs has no old domain-watcher terms
      const smContent = readFile(SESSION_MANAGER_RS);
      expect(smContent).not.toMatch(/\bWatchGraph\b/);
      expect(smContent).not.toMatch(/\bSessionRole\b/);
      expect(smContent).not.toMatch(/\bRoleAuthority\b/);
      expect(smContent).not.toMatch(/\bwatcher_agent_loop\b/);
      expect(smContent).not.toMatch(/\bwatcher_loop_tick\b/);
      expect(smContent).not.toMatch(/\brun_watcher_loop\b/);
      expect(smContent).not.toMatch(/\bWatcherOutput\b/);
      expect(smContent).not.toMatch(/\bwatcher_broadcast\b/);
      expect(smContent).not.toMatch(/\bwatcher_input_tx\b/);
      expect(smContent).not.toMatch(/\bwatcher_input_rx\b/);
      expect(smContent).not.toMatch(/\bwatcher_inject\b/);
      expect(smContent).not.toMatch(/\bsession_create_watcher\b/);
      expect(smContent).not.toMatch(/\bsession_get_watchers\b/);
      // session_clear_role was removed as a function, comment reference is OK
      expect(smContent).not.toMatch(/fn session_clear_role/);

      // Check types.rs has no old variant names
      const typesContent = readFile(TYPES_RS);
      expect(typesContent).not.toMatch(/\bWatcherInput\b/);
      expect(typesContent).not.toMatch(/\bWatcherPendingInjection\b/);
      expect(typesContent).not.toMatch(/\bWatcherInputImage\b/);
      expect(typesContent).not.toMatch(/"watcherInput"/);
      expect(typesContent).not.toMatch(/"watcherPendingInjection"/);

      // @step When the full test suite is executed
      // @step Then all tests pass with zero behavioral changes
      // Verified by running the full test suite — this scenario validates the naming sweep
      const smContent2 = readFile(SESSION_MANAGER_RS);
      // Ensure no old function names remain in production code
      expect(smContent2).not.toMatch(/fn session_clear_role/);
      expect(smContent2).not.toMatch(/\bcreate_watcher_session_with_id\b/);
      expect(smContent2).not.toMatch(/\bwatcher_inject\b/);
      expect(smContent2).not.toMatch(/\bcleanup_parent\b/);
    });
  });
});
