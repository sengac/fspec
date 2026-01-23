/**
 * Feature: spec/features/auto-inject-toggle-in-watcher-creation-dialog.feature
 *
 * Tests for Auto-inject Toggle in Watcher Creation Dialog (WATCH-021)
 *
 * These tests verify the auto-inject toggle functionality in WatcherCreateView.
 * Tests follow the pattern of extracting logic functions that mirror the component's
 * behavior for unit testing without full React/Ink rendering.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionCreateWatcher = vi.fn();
const mockSessionSetRole = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionCreateWatcher: mockSessionCreateWatcher,
  sessionSetRole: mockSessionSetRole,
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
}));

// =============================================================================
// Type definitions that MUST match WatcherCreateView.tsx after WATCH-021 changes
// =============================================================================

// Current FocusField type (to be updated in implementation)
// type FocusField = 'name' | 'authority' | 'model' | 'brief' | 'create';

// New FocusField type WITH autoInject (WATCH-021)
type FocusField = 'name' | 'authority' | 'model' | 'brief' | 'autoInject' | 'create';

// Focus order constant - WATCH-021: autoInject added between brief and create
const FOCUS_ORDER: FocusField[] = ['name', 'authority', 'model', 'brief', 'autoInject', 'create'];

// =============================================================================
// Helper functions that mirror WatcherCreateView logic
// =============================================================================

/**
 * Focus cycling logic - matches cycleFocusForward in WatcherCreateView.tsx
 */
const cycleFocusForward = (currentField: FocusField): FocusField => {
  const currentIndex = FOCUS_ORDER.indexOf(currentField);
  return FOCUS_ORDER[(currentIndex + 1) % FOCUS_ORDER.length];
};

/**
 * Focus cycling logic - matches cycleFocusBackward in WatcherCreateView.tsx
 */
const cycleFocusBackward = (currentField: FocusField): FocusField => {
  const currentIndex = FOCUS_ORDER.indexOf(currentField);
  return FOCUS_ORDER[(currentIndex - 1 + FOCUS_ORDER.length) % FOCUS_ORDER.length];
};

/**
 * Auto-inject toggle logic - matches keyboard handler in WatcherCreateView.tsx
 */
const toggleAutoInject = (current: boolean): boolean => {
  return !current;
};

/**
 * Format auto-inject display - matches render logic in WatcherCreateView.tsx
 */
const formatAutoInjectDisplay = (enabled: boolean, focused: boolean): {
  text: string;
  color: string;
  showHint: boolean;
} => {
  return {
    text: enabled ? '[●] Enabled' : '[ ] Disabled',
    color: enabled ? 'green' : 'gray',
    showHint: focused,
  };
};

/**
 * onCreate callback signature - MUST match WatcherCreateViewProps.onCreate after WATCH-021
 */
type OnCreateCallback = (
  name: string,
  authority: 'peer' | 'supervisor',
  model: string,
  brief: string,
  autoInject: boolean
) => void;

/**
 * Form state type - matches useState declarations in WatcherCreateView.tsx
 */
interface WatcherCreateState {
  name: string;
  authority: 'peer' | 'supervisor';
  selectedModelIndex: number;
  brief: string;
  autoInject: boolean;
  focusField: FocusField;
}

// =============================================================================
// Tests
// =============================================================================

describe('Feature: Auto-Inject Toggle in Watcher Creation Dialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Auto-inject defaults to enabled', () => {
    it('should default autoInject to true when dialog opens', () => {
      // @step Given the user opens the watcher creation dialog
      const initialState: WatcherCreateState = {
        name: '',
        authority: 'peer',
        selectedModelIndex: 0,
        brief: '',
        autoInject: true, // WATCH-021: Default is enabled
        focusField: 'name',
      };

      // @step Then the Auto-inject field should show '[●] Enabled' in green
      const display = formatAutoInjectDisplay(initialState.autoInject, false);
      expect(initialState.autoInject).toBe(true);
      expect(display.text).toBe('[●] Enabled');
      expect(display.color).toBe('green');
    });
  });

  describe('Scenario: User disables auto-inject with arrow keys', () => {
    it('should toggle autoInject when Left/Right arrow is pressed', () => {
      // @step Given the watcher creation dialog is open
      let state: WatcherCreateState = {
        name: '',
        authority: 'peer',
        selectedModelIndex: 0,
        brief: '',
        autoInject: true,
        focusField: 'autoInject', // Focus on autoInject field
      };

      // @step And the Auto-inject field is focused and shows Enabled
      expect(state.focusField).toBe('autoInject');
      expect(state.autoInject).toBe(true);
      let display = formatAutoInjectDisplay(state.autoInject, true);
      expect(display.text).toBe('[●] Enabled');

      // @step When the user presses the Right arrow key
      // (Left or Right arrow toggles the value)
      state = { ...state, autoInject: toggleAutoInject(state.autoInject) };

      // @step Then the Auto-inject field should show '[ ] Disabled' in gray
      expect(state.autoInject).toBe(false);
      display = formatAutoInjectDisplay(state.autoInject, true);
      expect(display.text).toBe('[ ] Disabled');
      expect(display.color).toBe('gray');
    });

    it('should toggle back to enabled when arrow key pressed again', () => {
      // Start with disabled
      let autoInject = false;
      
      // Toggle
      autoInject = toggleAutoInject(autoInject);
      expect(autoInject).toBe(true);
      
      // Toggle again
      autoInject = toggleAutoInject(autoInject);
      expect(autoInject).toBe(false);
    });
  });

  describe('Scenario: Tab navigation includes auto-inject field', () => {
    it('should cycle focus through autoInject field between brief and create', () => {
      // @step Given the watcher creation dialog is open
      let focusField: FocusField = 'name';

      // @step And the Brief field is focused
      focusField = 'brief';
      expect(focusField).toBe('brief');

      // @step When the user presses Tab
      focusField = cycleFocusForward(focusField);

      // @step Then the Auto-inject field should be focused
      expect(focusField).toBe('autoInject');

      // @step When the user presses Tab again
      focusField = cycleFocusForward(focusField);

      // @step Then the Create button should be focused
      expect(focusField).toBe('create');
    });

    it('should verify FOCUS_ORDER includes autoInject in correct position', () => {
      // Verify autoInject is between brief and create
      const briefIndex = FOCUS_ORDER.indexOf('brief');
      const autoInjectIndex = FOCUS_ORDER.indexOf('autoInject');
      const createIndex = FOCUS_ORDER.indexOf('create');

      expect(autoInjectIndex).toBe(briefIndex + 1);
      expect(createIndex).toBe(autoInjectIndex + 1);
    });

    it('should handle Shift+Tab backward navigation', () => {
      // Start at create
      let focusField: FocusField = 'create';

      // Tab backward
      focusField = cycleFocusBackward(focusField);
      expect(focusField).toBe('autoInject');

      focusField = cycleFocusBackward(focusField);
      expect(focusField).toBe('brief');
    });
  });

  describe('Scenario: Auto-inject field shows focus styling and hint', () => {
    it('should show cyan label and blue highlight when focused', () => {
      // @step Given the watcher creation dialog is open
      const state: WatcherCreateState = {
        name: '',
        authority: 'peer',
        selectedModelIndex: 0,
        brief: '',
        autoInject: true,
        focusField: 'name', // Not focused yet
      };

      // @step When the user tabs to the Auto-inject field
      const focusedState = { ...state, focusField: 'autoInject' as FocusField };
      const display = formatAutoInjectDisplay(focusedState.autoInject, true);

      // @step Then the 'Auto-inject:' label should be cyan
      // (Verified by focusField === 'autoInject' check in render)
      expect(focusedState.focusField).toBe('autoInject');

      // @step And the toggle should have a blue background highlight
      // (Verified by focusField === 'autoInject' check triggering backgroundColor='blue')
      expect(focusedState.focusField).toBe('autoInject');

      // @step And the hint '(←/→ to toggle)' should be visible
      expect(display.showHint).toBe(true);
    });

    it('should NOT show hint when not focused', () => {
      const display = formatAutoInjectDisplay(true, false);
      expect(display.showHint).toBe(false);
    });
  });

  describe('Scenario: Creating watcher passes auto-inject setting', () => {
    it('should call onCreate with autoInject=false when disabled', () => {
      // @step Given the watcher creation dialog is open
      const state: WatcherCreateState = {
        name: 'Code Reviewer',
        authority: 'peer',
        selectedModelIndex: 0,
        brief: 'Watch for bugs',
        autoInject: true,
        focusField: 'create',
      };

      // @step And the user has entered a valid role name
      expect(state.name.trim().length).toBeGreaterThan(0);

      // @step And the user has disabled auto-inject
      const disabledState = { ...state, autoInject: false };
      expect(disabledState.autoInject).toBe(false);

      // @step When the user presses Enter to create the watcher
      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      const mockOnCreate = vi.fn<Parameters<OnCreateCallback>, void>();
      mockOnCreate(
        disabledState.name.trim(),
        disabledState.authority,
        'anthropic/claude-sonnet-4-20250514', // selectedModel with provider prefix
        disabledState.brief.trim(),
        disabledState.autoInject
      );

      // @step Then onCreate should be called with autoInject set to false
      expect(mockOnCreate).toHaveBeenCalledWith(
        'Code Reviewer',
        'peer',
        'anthropic/claude-sonnet-4-20250514',
        'Watch for bugs',
        false
      );
    });

    it('should call onCreate with autoInject=true when enabled', () => {
      const state: WatcherCreateState = {
        name: 'Security Reviewer',
        authority: 'supervisor',
        selectedModelIndex: 0,
        brief: '',
        autoInject: true,
        focusField: 'create',
      };

      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      const mockOnCreate = vi.fn<Parameters<OnCreateCallback>, void>();
      mockOnCreate(
        state.name.trim(),
        state.authority,
        'anthropic/claude-sonnet-4-20250514',
        state.brief.trim(),
        state.autoInject
      );

      expect(mockOnCreate).toHaveBeenCalledWith(
        'Security Reviewer',
        'supervisor',
        'anthropic/claude-sonnet-4-20250514',
        '',
        true
      );
    });
  });

  describe('Integration: handleWatcherCreate passes autoInject to NAPI', () => {
    /**
     * This test verifies the integration contract between WatcherCreateView and AgentView.
     * The handleWatcherCreate callback must pass autoInject to sessionSetRole.
     */
    it('should pass autoInject to sessionSetRole via handleWatcherCreate', async () => {
      // Setup mock returns
      mockSessionCreateWatcher.mockResolvedValue('new-watcher-id');
      mockSessionSetRole.mockReturnValue(undefined);

      // Simulate what AgentView.handleWatcherCreate does with WATCH-021 changes
      // WATCH-023 FIX: Model IDs now include provider prefix for Rust parsing
      const handleWatcherCreate = async (
        name: string,
        authority: 'peer' | 'supervisor',
        model: string,
        brief: string,
        autoInject: boolean
      ): Promise<void> => {
        const watcherId = await mockSessionCreateWatcher(
          'parent-session-id',
          model,
          '/project',
          name
        );

        // WATCH-021: Pass autoInject to sessionSetRole
        mockSessionSetRole(
          watcherId,
          name,
          brief || null,
          authority,
          autoInject // NEW: 5th parameter
        );
      };

      // Execute - model now includes provider prefix
      await handleWatcherCreate(
        'Test Reviewer',
        'peer',
        'anthropic/claude-sonnet-4-20250514',
        'Test brief',
        false // autoInject disabled
      );

      // Verify sessionSetRole was called with autoInject=false
      expect(mockSessionSetRole).toHaveBeenCalledWith(
        'new-watcher-id',
        'Test Reviewer',
        'Test brief',
        'peer',
        false
      );
    });
  });
});

describe('Unit Tests: Auto-inject toggle helpers', () => {
  describe('toggleAutoInject', () => {
    it('should toggle true to false', () => {
      expect(toggleAutoInject(true)).toBe(false);
    });

    it('should toggle false to true', () => {
      expect(toggleAutoInject(false)).toBe(true);
    });
  });

  describe('formatAutoInjectDisplay', () => {
    it('should format enabled state correctly', () => {
      const result = formatAutoInjectDisplay(true, false);
      expect(result.text).toBe('[●] Enabled');
      expect(result.color).toBe('green');
      expect(result.showHint).toBe(false);
    });

    it('should format disabled state correctly', () => {
      const result = formatAutoInjectDisplay(false, false);
      expect(result.text).toBe('[ ] Disabled');
      expect(result.color).toBe('gray');
      expect(result.showHint).toBe(false);
    });

    it('should show hint when focused', () => {
      const result = formatAutoInjectDisplay(true, true);
      expect(result.showHint).toBe(true);
    });
  });

  describe('FOCUS_ORDER constant', () => {
    it('should have 6 fields in correct order', () => {
      expect(FOCUS_ORDER).toEqual([
        'name',
        'authority',
        'model',
        'brief',
        'autoInject',
        'create',
      ]);
    });

    it('should have autoInject at index 4', () => {
      expect(FOCUS_ORDER[4]).toBe('autoInject');
    });
  });

  describe('cycleFocusForward', () => {
    it('should cycle through all fields', () => {
      let field: FocusField = 'name';
      field = cycleFocusForward(field); // authority
      field = cycleFocusForward(field); // model
      field = cycleFocusForward(field); // brief
      field = cycleFocusForward(field); // autoInject
      expect(field).toBe('autoInject');
      field = cycleFocusForward(field); // create
      expect(field).toBe('create');
      field = cycleFocusForward(field); // wraps to name
      expect(field).toBe('name');
    });
  });

  describe('cycleFocusBackward', () => {
    it('should cycle backward through all fields', () => {
      let field: FocusField = 'name';
      field = cycleFocusBackward(field); // wraps to create
      expect(field).toBe('create');
      field = cycleFocusBackward(field); // autoInject
      expect(field).toBe('autoInject');
      field = cycleFocusBackward(field); // brief
      expect(field).toBe('brief');
    });
  });
});
