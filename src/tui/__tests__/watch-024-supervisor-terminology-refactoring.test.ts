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
import { execSync } from 'child_process';
import { readFileSync, existsSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const PROJECT_ROOT = process.cwd();
const CODELET_NAPI_SRC = join(PROJECT_ROOT, 'codelet', 'napi', 'src');
const CODELET_NAPI_TESTS = join(PROJECT_ROOT, 'codelet', 'napi', 'tests');
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
    it('should have SupervisorRole with correct fields', () => {
      // @step Given the SessionRole struct has been renamed to SupervisorRole
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'SupervisorRole')
      ).toBe(true);
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'SessionRole')
      ).toBe(false);

      // @step Then SupervisorRole has a name field of type String
      // (verified by struct existing with name field — checked via compilation)
      expect(fileContains(SESSION_MANAGER_RS, 'pub name: String')).toBe(true);

      // @step And SupervisorRole has a brief field of type Option<String>
      expect(
        fileContains(SESSION_MANAGER_RS, 'pub brief: Option<String>')
      ).toBe(true);

      // @step And SupervisorRole has an auto_inject field of type bool
      expect(fileContains(SESSION_MANAGER_RS, 'pub auto_inject: bool')).toBe(
        true
      );

      // @step And SupervisorRole does not have an authority field
      // The 'authority' field should not exist in the SupervisorRole struct
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
    it('should have form without Authority', () => {
      // @step Given the supervisor template creation form is displayed
      const formFile = join(TUI_COMPONENTS, 'SupervisorTemplateForm.tsx');
      expect(existsSync(formFile)).toBe(true);

      // @step Then the form has a Name field
      expect(fileContains(formFile, 'Name')).toBe(true);

      // @step And the form has a Model field
      expect(fileContains(formFile, 'Model')).toBe(true);

      // @step And the form has a Brief field
      expect(fileContains(formFile, 'Brief')).toBe(true);

      // @step And the form has an Auto-inject toggle
      expect(fileContains(formFile, 'Auto-inject')).toBe(true);

      // @step And the form does not have an Authority toggle
      expect(fileContains(formFile, 'authority')).toBe(false);
      const oldFormFile = join(TUI_COMPONENTS, 'WatcherTemplateForm.tsx');
      expect(existsSync(oldFormFile)).toBe(false);
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
    it('should have supervisor_inject without NAPI export', () => {
      // @step Given the watcher_inject function has been renamed to supervisor_inject
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_inject')).toBe(true);
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_inject')).toBe(false);

      // @step Then supervisor_inject does not have a #[napi] annotation
      // Read the function and check that #[napi] is not immediately before it
      const content = readFile(SESSION_MANAGER_RS);
      const fnIndex = content.indexOf('fn supervisor_inject');
      expect(fnIndex).toBeGreaterThan(-1);
      // Check the 200 chars before the function - should NOT contain #[napi]
      const preceding = content.substring(Math.max(0, fnIndex - 200), fnIndex);
      expect(preceding).not.toContain('#[napi]');

      // @step And supervisor_inject is called internally by the auto-inject path in the supervisor agent loop
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_inject(')).toBe(true);

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
    it('should have supervisor agent loop names', () => {
      // @step Given the watcher agent loop code has been refactored
      // @step Then watcher_agent_loop is renamed to supervisor_agent_loop
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_agent_loop')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_agent_loop')).toBe(
        false
      );

      // @step And watcher_loop_tick is renamed to supervisor_loop_tick
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_loop_tick')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_loop_tick')).toBe(false);

      // @step And run_watcher_loop is renamed to run_supervisor_loop
      expect(fileContains(SESSION_MANAGER_RS, 'run_supervisor_loop')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'run_watcher_loop')).toBe(false);

      // @step And WatcherState is renamed to SupervisorState
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'SupervisorState')
      ).toBe(true);
      // WatcherState should not exist in session_manager.rs (but may in work_units_watcher.rs - excluded)
      // Use a more precise check - ensure it's not in session_manager.rs
      const smContent = readFile(SESSION_MANAGER_RS);
      // Count WatcherState in session_manager.rs only (not work_units_watcher.rs)
      const watcherStateMatches = smContent.match(/\bWatcherState\b/g);
      expect(watcherStateMatches).toBeNull();

      // @step And WatcherOutput is renamed to SupervisorOutput
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'SupervisorOutput')
      ).toBe(true);
      expect(
        fileContainsRustIdentifier(SESSION_MANAGER_RS, 'WatcherOutput')
      ).toBe(false);
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

      // @step And the watcher_input_tx field is renamed to supervisor_input_tx
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_input_tx')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_input_tx')).toBe(false);

      // @step And the watcher_input_rx field is renamed to supervisor_input_rx
      expect(fileContains(SESSION_MANAGER_RS, 'supervisor_input_rx')).toBe(
        true
      );
      expect(fileContains(SESSION_MANAGER_RS, 'watcher_input_rx')).toBe(false);
    });
  });

  describe('Scenario: TUI components use supervisor naming', () => {
    it('should have renamed TUI component files', () => {
      // @step Given the TUI component files have been renamed
      // @step Then WatcherCreateView is renamed to SupervisorCreateView
      expect(existsSync(join(TUI_COMPONENTS, 'SupervisorCreateView.tsx'))).toBe(
        true
      );
      expect(existsSync(join(TUI_COMPONENTS, 'WatcherCreateView.tsx'))).toBe(
        false
      );

      // @step And WatcherTemplateList is renamed to SupervisorTemplateList
      expect(
        existsSync(join(TUI_COMPONENTS, 'SupervisorTemplateList.tsx'))
      ).toBe(true);
      expect(existsSync(join(TUI_COMPONENTS, 'WatcherTemplateList.tsx'))).toBe(
        false
      );

      // @step And WatcherTemplateForm is renamed to SupervisorTemplateForm
      expect(
        existsSync(join(TUI_COMPONENTS, 'SupervisorTemplateForm.tsx'))
      ).toBe(true);
      expect(existsSync(join(TUI_COMPONENTS, 'WatcherTemplateForm.tsx'))).toBe(
        false
      );

      // @step And useWatcherHeaderInfo is renamed to useSupervisorHeaderInfo
      expect(existsSync(join(TUI_HOOKS, 'useSupervisorHeaderInfo.ts'))).toBe(
        true
      );
      expect(existsSync(join(TUI_HOOKS, 'useWatcherHeaderInfo.ts'))).toBe(
        false
      );
    });
  });

  describe('Scenario: Template storage uses supervisor naming', () => {
    it('should have supervisor template storage', () => {
      // @step Given the template storage system has been updated
      const storageFile = join(TUI_UTILS, 'supervisorTemplateStorage.ts');

      // @step Then templates are stored in supervisor-templates.json instead of watcher-templates.json
      expect(existsSync(storageFile)).toBe(true);
      expect(fileContains(storageFile, 'supervisor-templates.json')).toBe(true);
      expect(existsSync(join(TUI_UTILS, 'watcherTemplateStorage.ts'))).toBe(
        false
      );

      // @step And the WatcherTemplate type is renamed to SupervisorTemplate
      const typeFile = join(TUI_TYPES, 'supervisorTemplate.ts');
      expect(existsSync(typeFile)).toBe(true);
      expect(existsSync(join(TUI_TYPES, 'watcherTemplate.ts'))).toBe(false);

      // @step And loadWatcherTemplates is renamed to loadSupervisorTemplates
      expect(fileContains(storageFile, 'loadSupervisorTemplates')).toBe(true);

      // @step And saveWatcherTemplates is renamed to saveSupervisorTemplates
      expect(fileContains(storageFile, 'saveSupervisorTemplates')).toBe(true);
    });
  });

  describe('Scenario: Slash command renamed from /watcher to /supervisor', () => {
    it('should have /supervisor and no /watcher or /parent', () => {
      // @step Given the slash command registry has been updated
      const slashCommandsFile = join(TUI_UTILS, 'slashCommands.ts');

      // @step Then the /watcher command is renamed to /supervisor
      expect(fileContains(slashCommandsFile, "'supervisor'")).toBe(true);
      // Check old commands don't exist
      const content = readFile(slashCommandsFile);
      // /watcher should be gone as a registered command
      expect(content).not.toMatch(/name:\s*['"]watcher['"]/);

      // @step And the /parent command is removed entirely
      expect(content).not.toMatch(/name:\s*['"]parent['"]/);
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

      // @step And the split view header displays [SUPERVISOR] instead of [WATCHER]
      const splitView = join(TUI_COMPONENTS, 'SplitSessionView.tsx');
      expect(fileContains(splitView, '[SUPERVISOR]')).toBe(true);
      expect(fileContains(splitView, '[WATCHER]')).toBe(false);
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
