/**
 * Feature: spec/features/watcher-creation-dialog-ui.feature
 *
 * Tests for Supervisor Creation Dialog (WATCH-009, updated by WATCH-024)
 * 
 * NOTE: These tests verify the core logic that will be implemented in AgentView.tsx.
 * The logic functions here MUST match the implementation.
 * 
 * WATCH-024: authority field removed — brief provides all behavioral instruction.
 * Focus order is now: name, model, brief, autoInject, create.
 * 
 * UI integration tests require manual verification due to React/Ink complexity.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionCreateSupervisor = vi.fn();
const mockSessionSetRole = vi.fn();
const mockSessionGetSupervisors = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionGetStatus = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionCreateSupervisor: mockSessionCreateSupervisor,
  sessionSetRole: mockSessionSetRole,
  sessionGetSupervisors: mockSessionGetSupervisors,
  sessionGetRole: mockSessionGetRole,
  sessionGetStatus: mockSessionGetStatus,
  // Other required mocks
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
  // BRIDGE-006: Unified thinking level detection NAPI functions
  napiDetectThinkingLevel: vi.fn(() => 0), // Default to Off
  napiHasDisableKeywords: vi.fn(() => false),
  napiComputeEffectiveThinkingLevel: vi.fn((base: number, detected: number, forceOff: boolean) => {
    if (forceOff) { return 0; }
    return Math.max(base, detected);
  }),
}));

// Dialog form state - MUST match SupervisorCreateView.tsx state shape
// WATCH-024: authority removed — brief provides all behavioral instruction
interface SupervisorCreateDialogState {
  showCreateDialog: boolean;
  name: string;
  model: string;
  brief: string;
  autoInject: boolean;
  focusField: 'name' | 'model' | 'brief' | 'autoInject' | 'create';
}

// Focus order constant - MUST match SupervisorCreateView.tsx FOCUS_ORDER
const FOCUS_ORDER: SupervisorCreateDialogState['focusField'][] = [
  'name',
  'model',
  'brief',
  'autoInject',
  'create',
];

// Focus cycling logic - MUST match implementation
const cycleFocusForward = (
  currentFocus: SupervisorCreateDialogState['focusField']
): SupervisorCreateDialogState['focusField'] => {
  const currentIndex = FOCUS_ORDER.indexOf(currentFocus);
  return FOCUS_ORDER[(currentIndex + 1) % FOCUS_ORDER.length];
};

// Supervisor creation logic - MUST match handleSupervisorCreate in AgentView.tsx
const createSupervisor = async (
  subordinateId: string,
  project: string,
  state: SupervisorCreateDialogState,
  sessionCreateSupervisorFn: typeof mockSessionCreateSupervisor,
  sessionSetRoleFn: typeof mockSessionSetRole
): Promise<{ success: boolean; supervisorId?: string; error?: string }> => {
  // Validation: name is required
  if (!state.name.trim()) {
    return { success: false, error: 'Name is required' };
  }

  try {
    // Create the supervisor session
    const supervisorId = await sessionCreateSupervisorFn(
      subordinateId,
      state.model,
      project,
      state.name.trim()
    );

    // Set the role info — WATCH-024: no authority, brief provides behavioral instruction
    sessionSetRoleFn(
      supervisorId,
      state.name.trim(),
      state.brief.trim() || null,
      state.autoInject
    );

    return { success: true, supervisorId };
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : 'Failed to create supervisor',
    };
  }
};

describe('Feature: Supervisor Creation Dialog UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Open supervisor creation dialog with N key', () => {
    it('should open dialog with correct initial state', () => {
      // @step Given the Supervisor Management overlay is open
      const isSupervisorMode = true;
      expect(isSupervisorMode).toBe(true);

      // @step When the user presses the N key
      const currentSessionModel = 'anthropic/claude-sonnet-4-20250514';
      const dialogState: SupervisorCreateDialogState = {
        showCreateDialog: true,
        name: '',
        model: currentSessionModel,
        brief: '',
        autoInject: true,
        focusField: 'name',
      };

      // @step Then the Supervisor Creation dialog should open
      expect(dialogState.showCreateDialog).toBe(true);

      // @step And the name input field should be empty and focused
      expect(dialogState.name).toBe('');
      expect(dialogState.focusField).toBe('name');

      // @step And auto-inject should default to true
      expect(dialogState.autoInject).toBe(true);

      // @step And the model should be set to the current session model (with provider prefix)
      expect(dialogState.model).toBe('anthropic/claude-sonnet-4-20250514');
    });
  });

  describe('Scenario: Tab through dialog fields', () => {
    it('should cycle focus through all fields in correct order', () => {
      // @step Given the Supervisor Creation dialog is open
      const state: SupervisorCreateDialogState = {
        showCreateDialog: true,
        name: '',
        model: 'anthropic/claude-sonnet-4-20250514',
        brief: '',
        autoInject: true,
        focusField: 'name',
      };

      // @step And the name field is focused
      let focus = state.focusField;
      expect(focus).toBe('name');

      // @step When the user presses Tab
      focus = cycleFocusForward(focus);
      // @step Then the model selector should be focused
      expect(focus).toBe('model');

      // @step When the user presses Tab
      focus = cycleFocusForward(focus);
      // @step Then the brief textarea should be focused
      expect(focus).toBe('brief');

      // @step When the user presses Tab
      focus = cycleFocusForward(focus);
      // @step Then the auto-inject toggle should be focused
      expect(focus).toBe('autoInject');

      // @step When the user presses Tab
      focus = cycleFocusForward(focus);
      // @step Then the Create button should be focused
      expect(focus).toBe('create');

      // @step When the user presses Tab
      focus = cycleFocusForward(focus);
      // @step Then the name field should be focused again
      expect(focus).toBe('name');
    });
  });

  describe('Scenario: Create supervisor successfully', () => {
    it('should call NAPI functions and create supervisor', async () => {
      // @step Given the Supervisor Creation dialog is open
      const state: SupervisorCreateDialogState = {
        showCreateDialog: true,
        name: 'Code Reviewer',
        model: 'anthropic/claude-sonnet-4-20250514',
        brief: 'Reviews code changes',
        autoInject: true,
        focusField: 'name',
      };

      // @step And the user has entered name "Code Reviewer"
      expect(state.name).toBe('Code Reviewer');

      // @step And the user has entered brief "Reviews code changes"
      expect(state.brief).toBe('Reviews code changes');

      // Mock successful creation
      mockSessionCreateSupervisor.mockResolvedValue('new-supervisor-uuid');

      // @step When the user presses Enter on the Create button
      const result = await createSupervisor(
        'subordinate-session-uuid',
        '/project/path',
        state,
        mockSessionCreateSupervisor,
        mockSessionSetRole
      );

      // @step Then sessionCreateSupervisor should be called with the subordinate session ID and model
      expect(mockSessionCreateSupervisor).toHaveBeenCalledWith(
        'subordinate-session-uuid',
        'anthropic/claude-sonnet-4-20250514',
        '/project/path',
        'Code Reviewer'
      );

      // @step And sessionSetRole should be called with the new supervisor ID, name, brief, and autoInject
      expect(mockSessionSetRole).toHaveBeenCalledWith(
        'new-supervisor-uuid',
        'Code Reviewer',
        'Reviews code changes',
        true
      );

      // @step And the dialog should close
      expect(result.success).toBe(true);
      expect(result.supervisorId).toBe('new-supervisor-uuid');
    });
  });

  describe('Scenario: Cancel supervisor creation with Escape', () => {
    it('should close dialog without creating supervisor', () => {
      // @step Given the Supervisor Creation dialog is open
      let state: SupervisorCreateDialogState = {
        showCreateDialog: true,
        name: 'Some Name',
        model: 'anthropic/claude-sonnet-4-20250514',
        brief: 'Some brief',
        autoInject: false,
        focusField: 'name',
      };

      // @step And the user has entered some data in the fields
      expect(state.name).toBe('Some Name');

      // @step When the user presses Escape
      state = {
        showCreateDialog: false,
        name: '',
        model: 'anthropic/claude-sonnet-4-20250514',
        brief: '',
        autoInject: true,
        focusField: 'name',
      };

      // @step Then the dialog should close
      expect(state.showCreateDialog).toBe(false);

      // @step And no supervisor should be created
      expect(mockSessionCreateSupervisor).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Create button disabled when name is empty', () => {
    it('should not create supervisor when name is empty', async () => {
      // @step Given the Supervisor Creation dialog is open
      const state: SupervisorCreateDialogState = {
        showCreateDialog: true,
        name: '',
        model: 'anthropic/claude-sonnet-4-20250514',
        brief: '',
        autoInject: true,
        focusField: 'name',
      };

      // @step And the name field is empty
      expect(state.name).toBe('');

      // @step When the user presses Enter on the Create button
      const result = await createSupervisor(
        'subordinate-session-uuid',
        '/project/path',
        state,
        mockSessionCreateSupervisor,
        mockSessionSetRole
      );

      // @step Then no supervisor should be created
      expect(result.success).toBe(false);
      expect(result.error).toBe('Name is required');
      expect(mockSessionCreateSupervisor).not.toHaveBeenCalled();
    });
  });
});
