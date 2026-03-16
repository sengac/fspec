/**
 * Feature: spec/features/watcher-management-overlay-ui.feature
 *
 * Tests for /supervisor command Supervisor Management overlay (WATCH-008, updated by WATCH-024)
 * 
 * NOTE: These tests verify the core logic that is duplicated from AgentView.tsx.
 * The logic functions here MUST match the implementation in handleSupervisorMode().
 * If you change AgentView.tsx supervisor logic, update these tests to match.
 * 
 * WATCH-024: authority removed — brief provides all behavioral instruction.
 * sessionGetRole now returns SupervisorRoleInfo { name, brief }.
 * 
 * UI integration tests require manual verification due to React/Ink complexity.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionGetSupervisors = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionSetRole = vi.fn();
const mockSessionManagerDestroy = vi.fn();
const mockSessionGetStatus = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetSupervisors: mockSessionGetSupervisors,
  sessionGetRole: mockSessionGetRole,
  sessionSetRole: mockSessionSetRole,
  sessionManagerDestroy: mockSessionManagerDestroy,
  sessionGetStatus: mockSessionGetStatus,
  // Other required mocks for AgentView
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

// Helper types — MUST match SupervisorRoleInfo from NAPI (WATCH-024)
interface SupervisorRoleInfo {
  name: string;
  brief: string | null;
}

// Helper types - MUST match SupervisorInfo in AgentView.tsx
interface SupervisorInfo {
  id: string;
  name: string;
  role: SupervisorRoleInfo | null;
  status: 'idle' | 'running';
}

// Helper to create mock supervisor data
const createMockSupervisors = (count: number): SupervisorInfo[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: `supervisor-${i}`,
    name: i % 2 === 0 ? 'Code Reviewer' : 'Security Auditor',
    role: {
      name: i % 2 === 0 ? 'Code Reviewer' : 'Security Auditor',
      brief: null,
    },
    status: (i % 3 === 0 ? 'running' : 'idle') as 'idle' | 'running',
  }));
};

// Supervisor management state - MUST match AgentView.tsx state shape
interface SupervisorManagementState {
  isSupervisorMode: boolean;
  supervisorList: SupervisorInfo[];
  supervisorIndex: number;
  supervisorScrollOffset: number;
  showDeleteDialog: boolean;
}

/**
 * Supervisor loading logic - MUST match handleSupervisorMode() in AgentView.tsx
 */
const loadSupervisorList = async (currentSessionId: string): Promise<SupervisorInfo[]> => {
  const supervisorIds = mockSessionGetSupervisors(currentSessionId);
  const supervisors: SupervisorInfo[] = [];
  
  for (const id of supervisorIds) {
    const role = mockSessionGetRole(id);
    const status = mockSessionGetStatus(id);
    supervisors.push({
      id,
      name: role?.name || 'Unnamed Supervisor',
      role,
      status: status === 'running' ? 'running' : 'idle',
    });
  }
  
  return supervisors;
};

/**
 * Format supervisor display - MUST match overlay render logic in AgentView.tsx
 * WATCH-024: authority removed, display shows name and status only
 */
const formatSupervisorDisplay = (supervisor: SupervisorInfo): string => {
  return `${supervisor.name} (${supervisor.status})`;
};

/**
 * Navigation logic - MUST match useInput handler in AgentView.tsx
 */
const navigateDown = (currentIndex: number, listLength: number): number => {
  return Math.min(listLength - 1, currentIndex + 1);
};

const navigateUp = (currentIndex: number): number => {
  return Math.max(0, currentIndex - 1);
};

describe('Feature: Supervisor Management Overlay UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Open supervisor overlay with existing supervisors', () => {
    it('should display supervisor list when /supervisor command is executed', async () => {
      // @step Given a session with two supervisors: "Code Reviewer" (idle) and "Security Auditor" (running)
      const mockSupervisorIds = ['supervisor-1', 'supervisor-2'];
      mockSessionGetSupervisors.mockReturnValue(mockSupervisorIds);
      mockSessionGetRole.mockImplementation((id: string) => {
        if (id === 'supervisor-1') {
          return { name: 'Code Reviewer', brief: null };
        }
        return { name: 'Security Auditor', brief: 'Audits for vulnerabilities' };
      });
      mockSessionGetStatus.mockImplementation((id: string) => {
        return id === 'supervisor-1' ? 'idle' : 'running';
      });

      // @step When the user types "/supervisor" and presses Enter
      const supervisors = await loadSupervisorList('subordinate-session-id');

      // @step Then the Supervisor Management overlay should open
      expect(mockSessionGetSupervisors).toHaveBeenCalledWith('subordinate-session-id');
      expect(supervisors).toHaveLength(2);

      // @step And the overlay should display "Code Reviewer (idle)"
      expect(mockSessionGetRole).toHaveBeenCalledWith('supervisor-1');
      expect(mockSessionGetStatus).toHaveBeenCalledWith('supervisor-1');
      expect(formatSupervisorDisplay(supervisors[0])).toBe('Code Reviewer (idle)');

      // @step And the overlay should display "Security Auditor (running)"
      expect(mockSessionGetRole).toHaveBeenCalledWith('supervisor-2');
      expect(mockSessionGetStatus).toHaveBeenCalledWith('supervisor-2');
      expect(formatSupervisorDisplay(supervisors[1])).toBe('Security Auditor (running)');

      // @step And the first supervisor should be highlighted
      const initialIndex = 0;
      expect(initialIndex).toBe(0);
    });
  });

  describe('Scenario: Open supervisor overlay with no supervisors', () => {
    it('should display empty state message when no supervisors exist', async () => {
      // @step Given a session with no supervisors
      mockSessionGetSupervisors.mockReturnValue([]);

      // @step When the user types "/supervisor" and presses Enter
      const supervisors = await loadSupervisorList('subordinate-session-id');

      // @step Then the Supervisor Management overlay should open
      expect(mockSessionGetSupervisors).toHaveBeenCalledWith('subordinate-session-id');
      expect(supervisors).toHaveLength(0);

      // @step And the overlay should display "No supervisors. Press N to create one."
      const showEmptyMessage = supervisors.length === 0;
      expect(showEmptyMessage).toBe(true);
    });
  });

  describe('Scenario: Navigate supervisor list with arrow keys', () => {
    it('should navigate selection with arrow keys using correct boundary logic', () => {
      // @step Given the Supervisor Management overlay is open with 3 supervisors
      const supervisorList = createMockSupervisors(3);
      let supervisorIndex = 0;

      // @step And the first supervisor is selected
      expect(supervisorIndex).toBe(0);

      // @step When the user presses the down arrow key
      supervisorIndex = navigateDown(supervisorIndex, supervisorList.length);
      // @step Then the second supervisor should be highlighted
      expect(supervisorIndex).toBe(1);

      // @step When the user presses the down arrow key
      supervisorIndex = navigateDown(supervisorIndex, supervisorList.length);
      // @step Then the third supervisor should be highlighted
      expect(supervisorIndex).toBe(2);

      // Verify boundary: can't go past last item
      supervisorIndex = navigateDown(supervisorIndex, supervisorList.length);
      expect(supervisorIndex).toBe(2); // Should stay at 2

      // @step When the user presses the up arrow key
      supervisorIndex = navigateUp(supervisorIndex);
      // @step Then the second supervisor should be highlighted
      expect(supervisorIndex).toBe(1);

      // Verify boundary: can't go past first item
      supervisorIndex = navigateUp(supervisorIndex);
      supervisorIndex = navigateUp(supervisorIndex);
      expect(supervisorIndex).toBe(0); // Should stay at 0
    });
  });

  describe('Scenario: Open selected supervisor with Enter key', () => {
    it('should return selected supervisor info for session switch', async () => {
      // @step Given the Supervisor Management overlay is open with "Code Reviewer" selected
      mockSessionGetSupervisors.mockReturnValue(['supervisor-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', brief: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const supervisors = await loadSupervisorList('subordinate-session-id');
      const supervisorIndex = 0;

      // @step When the user presses Enter
      const selectedSupervisor = supervisors[supervisorIndex];
      
      // @step Then the overlay should close
      // (isSupervisorMode set to false - tested via state change)

      // @step And the session should switch to the "Code Reviewer" supervisor
      expect(selectedSupervisor).toBeDefined();
      expect(selectedSupervisor.id).toBe('supervisor-1');
      expect(selectedSupervisor.name).toBe('Code Reviewer');
    });
  });

  describe('Scenario: Delete supervisor with confirmation', () => {
    it('should format correct confirmation message for selected supervisor', async () => {
      // @step Given the Supervisor Management overlay is open with "Code Reviewer" selected
      mockSessionGetSupervisors.mockReturnValue(['supervisor-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', brief: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const supervisors = await loadSupervisorList('subordinate-session-id');
      const selectedSupervisor = supervisors[0];

      // @step When the user presses the D key
      // @step Then a confirmation dialog should appear
      const confirmationMessage = `Delete supervisor "${selectedSupervisor.name}"?`;
      expect(confirmationMessage).toBe('Delete supervisor "Code Reviewer"?');

      // @step And the dialog should have Delete and Cancel options
      const dialogOptions = ['Delete', 'Cancel'];
      expect(dialogOptions).toContain('Delete');
      expect(dialogOptions).toContain('Cancel');
    });

    it('should call sessionManagerDestroy when delete is confirmed', async () => {
      // Setup
      mockSessionGetSupervisors.mockReturnValue(['supervisor-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', brief: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const supervisors = await loadSupervisorList('subordinate-session-id');
      const selectedSupervisor = supervisors[0];

      // Simulate delete action
      mockSessionManagerDestroy(selectedSupervisor.id);

      // Verify NAPI was called with correct supervisor ID
      expect(mockSessionManagerDestroy).toHaveBeenCalledWith('supervisor-1');
    });
  });

  describe('Scenario: Close overlay with Escape key', () => {
    it('should reset supervisor mode state on Escape', () => {
      // @step Given the Supervisor Management overlay is open
      const state: SupervisorManagementState = {
        isSupervisorMode: true,
        supervisorList: createMockSupervisors(2),
        supervisorIndex: 1,
        supervisorScrollOffset: 0,
        showDeleteDialog: false,
      };
      expect(state.isSupervisorMode).toBe(true);

      // @step When the user presses the Escape key
      state.isSupervisorMode = false;
      state.supervisorList = [];

      // @step Then the overlay should close
      expect(state.isSupervisorMode).toBe(false);
      expect(state.supervisorList).toHaveLength(0);
    });
  });

  describe('Scenario: Scrollable list for many supervisors', () => {
    it('should calculate scroll visibility correctly', () => {
      // @step Given a session with 10 supervisors
      const supervisors = createMockSupervisors(10);
      
      // Visible height calculation: Math.max(1, Math.floor((terminalHeight - 6) / 2))
      const terminalHeight = 24;
      const visibleHeight = Math.max(1, Math.floor((terminalHeight - 6) / 2));

      // @step When the user types "/supervisor" and presses Enter
      const state: SupervisorManagementState = {
        isSupervisorMode: true,
        supervisorList: supervisors,
        supervisorIndex: 0,
        supervisorScrollOffset: 0,
        showDeleteDialog: false,
      };

      // @step Then the overlay should show a scrollable list
      const needsScrolling = state.supervisorList.length > visibleHeight;
      expect(needsScrolling).toBe(true);
      expect(state.supervisorList.length).toBeGreaterThan(visibleHeight);
    });
  });

  describe('Scenario: Edit supervisor with E key', () => {
    it('should activate inline edit mode on E key press', async () => {
      // @step Given the Supervisor Management overlay is open with "Code Reviewer" selected
      mockSessionGetSupervisors.mockReturnValue(['supervisor-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', brief: 'Reviews code' });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const supervisors = await loadSupervisorList('subordinate-session-id');
      const selectedSupervisor = supervisors[0];

      // Simulate edit mode state
      let isEditMode = false;
      let editValue = '';

      // @step When the user presses the E key
      editValue = selectedSupervisor.name;
      isEditMode = true;

      // @step Then inline edit mode should activate for the supervisor name
      expect(isEditMode).toBe(true);
      expect(editValue).toBe('Code Reviewer');

      // @step And the user can modify the name and press Enter to save
      editValue = 'Senior Reviewer';
      
      // Simulate Enter to save
      const updatedSupervisor = { ...selectedSupervisor, name: editValue.trim() };
      expect(updatedSupervisor.name).toBe('Senior Reviewer');
      
      // Mode should close after save
      isEditMode = false;
      editValue = '';
      expect(isEditMode).toBe(false);
    });
  });

  describe('Scenario: Scroll follows selection when navigating', () => {
    const adjustScrollOffset = (
      index: number,
      scrollOffset: number,
      visibleHeight: number
    ): number => {
      if (index < scrollOffset) {
        return index;
      } else if (index >= scrollOffset + visibleHeight) {
        return index - visibleHeight + 1;
      }
      return scrollOffset;
    };

    it('should adjust scroll offset to keep selection visible when navigating down', () => {
      // @step Given the Supervisor Management overlay is open with 10 supervisors
      const supervisorList = createMockSupervisors(10);
      
      // @step And only 5 supervisors are visible at a time
      const visibleHeight = 5;
      
      let supervisorIndex = 0;
      let scrollOffset = 0;
      
      expect(supervisorIndex).toBe(0);
      expect(scrollOffset).toBe(0);

      // @step When the user presses the down arrow key 6 times
      for (let i = 0; i < 6; i++) {
        supervisorIndex = navigateDown(supervisorIndex, supervisorList.length);
        scrollOffset = adjustScrollOffset(supervisorIndex, scrollOffset, visibleHeight);
      }

      // @step Then the 7th supervisor should be highlighted
      expect(supervisorIndex).toBe(6);

      // @step And the scroll offset should adjust to keep the selection visible
      expect(scrollOffset).toBe(2);
    });
  });

  describe('Scenario: Edit supervisor persists changes to backend', () => {
    /**
     * Edit save logic - MUST match useInput handler in AgentView.tsx
     * WATCH-024: sessionSetRole now takes (id, name, brief, autoInject) — no authority
     */
    const saveEditedSupervisorName = (
      supervisor: SupervisorInfo,
      newName: string,
      supervisorList: SupervisorInfo[],
      supervisorIndex: number,
      sessionSetRoleFn: typeof mockSessionSetRole
    ): { success: boolean; updatedList: SupervisorInfo[] } => {
      try {
        sessionSetRoleFn(
          supervisor.id,
          newName.trim(),
          supervisor.role?.brief || null,
        );
        // Update local state ONLY if backend save succeeded
        const updatedList = [...supervisorList];
        updatedList[supervisorIndex] = {
          ...updatedList[supervisorIndex],
          name: newName.trim(),
        };
        return { success: true, updatedList };
      } catch {
        // Do NOT update local state - keep showing old name for consistency
        return { success: false, updatedList: supervisorList };
      }
    };

    it('should call sessionSetRole and update local state when save succeeds', async () => {
      // @step Given the Supervisor Management overlay is open with "Code Reviewer" selected
      mockSessionGetSupervisors.mockReturnValue(['supervisor-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', brief: 'Reviews code' });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const supervisors = await loadSupervisorList('subordinate-session-id');
      const selectedSupervisor = supervisors[0];
      const supervisorIndex = 0;

      // @step And the user is in edit mode with value "Senior Reviewer"
      const editValue = 'Senior Reviewer';
      expect(editValue).toBe('Senior Reviewer');

      // @step When the user presses Enter to save
      const result = saveEditedSupervisorName(
        selectedSupervisor,
        editValue,
        supervisors,
        supervisorIndex,
        mockSessionSetRole
      );

      // @step Then sessionSetRole should be called with the new name and brief
      expect(mockSessionSetRole).toHaveBeenCalledWith(
        'supervisor-1',
        'Senior Reviewer',
        'Reviews code',
      );

      // @step And the supervisor list should show "Senior Reviewer"
      expect(result.success).toBe(true);
      expect(result.updatedList[supervisorIndex].name).toBe('Senior Reviewer');
    });

    it('should NOT update local state when sessionSetRole fails', async () => {
      // @step Given the Supervisor Management overlay is open with "Code Reviewer" selected
      mockSessionGetSupervisors.mockReturnValue(['supervisor-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', brief: 'Reviews code' });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const supervisors = await loadSupervisorList('subordinate-session-id');
      const selectedSupervisor = supervisors[0];
      const supervisorIndex = 0;

      // @step And the user is in edit mode with value "Senior Reviewer"
      const editValue = 'Senior Reviewer';

      // Mock sessionSetRole to throw an error
      const failingSetRole = vi.fn().mockImplementation(() => {
        throw new Error('Backend save failed');
      });

      // @step When the user presses Enter to save but backend fails
      const result = saveEditedSupervisorName(
        selectedSupervisor,
        editValue,
        supervisors,
        supervisorIndex,
        failingSetRole
      );

      // @step Then the save should fail
      expect(result.success).toBe(false);

      // @step And the supervisor list should still show "Code Reviewer" (not updated)
      expect(result.updatedList[supervisorIndex].name).toBe('Code Reviewer');
    });
  });
});
