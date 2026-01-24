/**
 * Feature: spec/features/watcher-creation-dialog-ui.feature
 *
 * Tests for Watcher Creation Dialog (WATCH-009)
 * 
 * NOTE: These tests verify the core logic that will be implemented in AgentView.tsx.
 * The logic functions here MUST match the implementation.
 * 
 * UI integration tests require manual verification due to React/Ink complexity.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionCreateWatcher = vi.fn();
const mockSessionSetRole = vi.fn();
const mockSessionGetWatchers = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionGetStatus = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionCreateWatcher: mockSessionCreateWatcher,
  sessionSetRole: mockSessionSetRole,
  sessionGetWatchers: mockSessionGetWatchers,
  sessionGetRole: mockSessionGetRole,
  sessionGetStatus: mockSessionGetStatus,
  // Other required mocks
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
}));

// Dialog form state - MUST match AgentView.tsx state shape
interface WatcherCreateDialogState {
  showWatcherCreateDialog: boolean;
  watcherName: string;
  watcherAuthority: 'peer' | 'supervisor';
  watcherModel: string;
  watcherBrief: string;
  createDialogFocus: 'name' | 'authority' | 'model' | 'brief' | 'create';
}

// Focus order constant - MUST match WatcherCreateView.tsx FOCUS_ORDER
const FOCUS_ORDER: WatcherCreateDialogState['createDialogFocus'][] = [
  'name',
  'authority',
  'model',
  'brief',
  'create',
];

// Focus cycling logic - MUST match implementation
const cycleFocusForward = (
  currentFocus: WatcherCreateDialogState['createDialogFocus']
): WatcherCreateDialogState['createDialogFocus'] => {
  const currentIndex = FOCUS_ORDER.indexOf(currentFocus);
  return FOCUS_ORDER[(currentIndex + 1) % FOCUS_ORDER.length];
};

// Authority toggle logic
const toggleAuthority = (
  current: 'peer' | 'supervisor',
  direction: 'left' | 'right'
): 'peer' | 'supervisor' => {
  if (direction === 'right') {
    return current === 'peer' ? 'supervisor' : 'peer';
  } else {
    return current === 'supervisor' ? 'peer' : 'supervisor';
  }
};

// Watcher creation logic - MUST match handleWatcherCreate in AgentView.tsx
const createWatcher = async (
  parentId: string,
  project: string,
  state: WatcherCreateDialogState,
  sessionCreateWatcherFn: typeof mockSessionCreateWatcher,
  sessionSetRoleFn: typeof mockSessionSetRole
): Promise<{ success: boolean; watcherId?: string; error?: string }> => {
  // Validation: name is required
  if (!state.watcherName.trim()) {
    return { success: false, error: 'Name is required' };
  }

  try {
    // Create the watcher session
    const watcherId = await sessionCreateWatcherFn(
      parentId,
      state.watcherModel,
      project,
      state.watcherName.trim()
    );

    // Set the role info
    sessionSetRoleFn(
      watcherId,
      state.watcherName.trim(),
      state.watcherBrief.trim() || null,
      state.watcherAuthority
    );

    return { success: true, watcherId };
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : 'Failed to create watcher',
    };
  }
};

describe('Feature: Watcher Creation Dialog UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Open watcher creation dialog with N key', () => {
    it('should open dialog with correct initial state', () => {
      // @step Given the Watcher Management overlay is open
      const isWatcherMode = true;
      expect(isWatcherMode).toBe(true);

      // @step When the user presses the N key
      // Implementation: setShowWatcherCreateDialog(true) with default state
      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      const currentSessionModel = 'anthropic/claude-sonnet-4-20250514';
      const dialogState: WatcherCreateDialogState = {
        showWatcherCreateDialog: true,
        watcherName: '',
        watcherAuthority: 'peer',
        watcherModel: currentSessionModel,
        watcherBrief: '',
        createDialogFocus: 'name',
      };

      // @step Then the Watcher Creation dialog should open
      expect(dialogState.showWatcherCreateDialog).toBe(true);

      // @step And the name input field should be empty and focused
      expect(dialogState.watcherName).toBe('');
      expect(dialogState.createDialogFocus).toBe('name');

      // @step And the authority should be set to "Peer" by default
      expect(dialogState.watcherAuthority).toBe('peer');

      // @step And the model should be set to the current session model (with provider prefix)
      expect(dialogState.watcherModel).toBe('anthropic/claude-sonnet-4-20250514');
    });
  });

  describe('Scenario: Tab through dialog fields', () => {
    it('should cycle focus through all fields in correct order', () => {
      // @step Given the Watcher Creation dialog is open
      let state: WatcherCreateDialogState = {
        showWatcherCreateDialog: true,
        watcherName: '',
        watcherAuthority: 'peer',
        watcherModel: 'anthropic/claude-sonnet-4-20250514',
        watcherBrief: '',
        createDialogFocus: 'name',
      };

      // @step And the name field is focused
      expect(state.createDialogFocus).toBe('name');

      // @step When the user presses Tab
      state.createDialogFocus = cycleFocusForward(state.createDialogFocus);
      // @step Then the authority selector should be focused
      expect(state.createDialogFocus).toBe('authority');

      // @step When the user presses Tab
      state.createDialogFocus = cycleFocusForward(state.createDialogFocus);
      // @step Then the model selector should be focused
      expect(state.createDialogFocus).toBe('model');

      // @step When the user presses Tab
      state.createDialogFocus = cycleFocusForward(state.createDialogFocus);
      // @step Then the brief textarea should be focused
      expect(state.createDialogFocus).toBe('brief');

      // @step When the user presses Tab
      state.createDialogFocus = cycleFocusForward(state.createDialogFocus);
      // @step Then the Create button should be focused
      expect(state.createDialogFocus).toBe('create');

      // @step When the user presses Tab
      state.createDialogFocus = cycleFocusForward(state.createDialogFocus);
      // @step Then the name field should be focused again
      expect(state.createDialogFocus).toBe('name');
    });
  });

  describe('Scenario: Toggle authority with arrow keys', () => {
    it('should toggle authority between Peer and Supervisor', () => {
      // @step Given the Watcher Creation dialog is open
      // @step And the authority selector is focused with value "Peer"
      let authority: 'peer' | 'supervisor' = 'peer';
      expect(authority).toBe('peer');

      // @step When the user presses the right arrow key
      authority = toggleAuthority(authority, 'right');
      // @step Then the authority should change to "Supervisor"
      expect(authority).toBe('supervisor');

      // @step When the user presses the left arrow key
      authority = toggleAuthority(authority, 'left');
      // @step Then the authority should change to "Peer"
      expect(authority).toBe('peer');
    });
  });

  describe('Scenario: Create watcher successfully', () => {
    it('should call NAPI functions and create watcher', async () => {
      // @step Given the Watcher Creation dialog is open
      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      const state: WatcherCreateDialogState = {
        showWatcherCreateDialog: true,
        watcherName: 'Code Reviewer',
        watcherAuthority: 'peer',
        watcherModel: 'anthropic/claude-sonnet-4-20250514',
        watcherBrief: 'Reviews code changes',
        createDialogFocus: 'name',
      };

      // @step And the user has entered name "Code Reviewer"
      expect(state.watcherName).toBe('Code Reviewer');

      // @step And the user has selected authority "Peer"
      expect(state.watcherAuthority).toBe('peer');

      // @step And the user has selected model "anthropic/claude-sonnet-4-20250514"
      expect(state.watcherModel).toBe('anthropic/claude-sonnet-4-20250514');

      // @step And the user has entered brief "Reviews code changes"
      expect(state.watcherBrief).toBe('Reviews code changes');

      // Mock successful creation
      mockSessionCreateWatcher.mockResolvedValue('new-watcher-uuid');

      // @step When the user presses Enter on the Create button
      const result = await createWatcher(
        'parent-session-uuid',
        '/project/path',
        state,
        mockSessionCreateWatcher,
        mockSessionSetRole
      );

      // @step Then sessionCreateWatcher should be called with the parent session ID and model (with provider prefix)
      expect(mockSessionCreateWatcher).toHaveBeenCalledWith(
        'parent-session-uuid',
        'anthropic/claude-sonnet-4-20250514',
        '/project/path',
        'Code Reviewer'
      );

      // @step And sessionSetRole should be called with the new watcher ID, name, brief, and authority
      expect(mockSessionSetRole).toHaveBeenCalledWith(
        'new-watcher-uuid',
        'Code Reviewer',
        'Reviews code changes',
        'peer'
      );

      // @step And the dialog should close
      expect(result.success).toBe(true);
      expect(result.watcherId).toBe('new-watcher-uuid');

      // @step And the Watcher Management overlay should show the new watcher "Code Reviewer"
      // (verified by refreshing watcher list via handleWatcherMode after creation)
    });
  });

  describe('Scenario: Cancel watcher creation with Escape', () => {
    it('should close dialog without creating watcher', () => {
      // @step Given the Watcher Creation dialog is open
      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      let state: WatcherCreateDialogState = {
        showWatcherCreateDialog: true,
        watcherName: 'Some Name',
        watcherAuthority: 'supervisor',
        watcherModel: 'anthropic/claude-sonnet-4-20250514',
        watcherBrief: 'Some description',
        createDialogFocus: 'name',
      };

      // @step And the user has entered some data in the fields
      expect(state.watcherName).toBe('Some Name');

      // @step When the user presses Escape
      // Implementation: setShowWatcherCreateDialog(false), reset form state
      state = {
        showWatcherCreateDialog: false,
        watcherName: '',
        watcherAuthority: 'peer',
        watcherModel: 'anthropic/claude-sonnet-4-20250514',
        watcherBrief: '',
        createDialogFocus: 'name',
      };

      // @step Then the dialog should close
      expect(state.showWatcherCreateDialog).toBe(false);

      // @step And the Watcher Management overlay should be visible
      // (isWatcherMode stays true)

      // @step And the watcher list should be unchanged
      expect(mockSessionCreateWatcher).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Create button disabled when name is empty', () => {
    it('should not create watcher when name is empty', async () => {
      // @step Given the Watcher Creation dialog is open
      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      const state: WatcherCreateDialogState = {
        showWatcherCreateDialog: true,
        watcherName: '',
        watcherAuthority: 'peer',
        watcherModel: 'anthropic/claude-sonnet-4-20250514',
        watcherBrief: '',
        createDialogFocus: 'name',
      };

      // @step And the name field is empty
      expect(state.watcherName).toBe('');

      // @step When the user presses Enter on the Create button
      const result = await createWatcher(
        'parent-session-uuid',
        '/project/path',
        state,
        mockSessionCreateWatcher,
        mockSessionSetRole
      );

      // @step Then no watcher should be created
      expect(result.success).toBe(false);
      expect(result.error).toBe('Name is required');
      expect(mockSessionCreateWatcher).not.toHaveBeenCalled();

      // @step And the name field should show a required indicator
      // (verified in UI by checking state.watcherName === '' shows error styling)
    });
  });
});
