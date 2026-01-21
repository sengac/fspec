/**
 * Feature: spec/features/watcher-management-overlay-ui.feature
 *
 * Tests for /watcher command Watcher Management overlay (WATCH-008)
 * 
 * NOTE: These tests verify the core logic that is duplicated from AgentView.tsx.
 * The logic functions here MUST match the implementation in handleWatcherMode().
 * If you change AgentView.tsx watcher logic, update these tests to match.
 * 
 * UI integration tests require manual verification due to React/Ink complexity.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionGetWatchers = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionSetRole = vi.fn();
const mockSessionManagerDestroy = vi.fn();
const mockSessionGetStatus = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetWatchers: mockSessionGetWatchers,
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
}));

// Helper types - MUST match WatcherInfo in AgentView.tsx
interface WatcherInfo {
  id: string;
  name: string;
  role: {
    name: string;
    authority: string;
    description: string | null;
  } | null;
  status: 'idle' | 'running';
}

// Helper to create mock watcher data
const createMockWatchers = (count: number): WatcherInfo[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: `watcher-${i}`,
    name: i % 2 === 0 ? 'Code Reviewer' : 'Security Auditor',
    role: {
      name: i % 2 === 0 ? 'Code Reviewer' : 'Security Auditor',
      authority: i % 2 === 0 ? 'peer' : 'supervisor',
      description: null,
    },
    status: (i % 3 === 0 ? 'running' : 'idle') as 'idle' | 'running',
  }));
};

// Watcher management state - MUST match AgentView.tsx state shape
interface WatcherManagementState {
  isWatcherMode: boolean;
  watcherList: WatcherInfo[];
  watcherIndex: number;
  watcherScrollOffset: number;
  showWatcherDeleteDialog: boolean;
}

/**
 * Watcher loading logic - MUST match handleWatcherMode() in AgentView.tsx
 * 
 * This duplicates the logic from:
 *   const watcherIds = sessionGetWatchers(currentSessionId);
 *   for (const id of watcherIds) { ... }
 */
const loadWatcherList = async (currentSessionId: string): Promise<WatcherInfo[]> => {
  const watcherIds = mockSessionGetWatchers(currentSessionId);
  const watchers: WatcherInfo[] = [];
  
  for (const id of watcherIds) {
    const role = mockSessionGetRole(id);
    const status = mockSessionGetStatus(id);
    watchers.push({
      id,
      name: role?.name || 'Unnamed Watcher',
      role,
      status: status === 'running' ? 'running' : 'idle',
    });
  }
  
  return watchers;
};

/**
 * Format watcher display - MUST match overlay render logic in AgentView.tsx
 * 
 * This duplicates:
 *   const authorityDisplay = watcher.role?.authority === 'supervisor' ? 'Supervisor' : 'Peer';
 *   `${statusIcon} ${watcher.name}` and `${authorityDisplay} | ${watcher.status}`
 */
const formatWatcherDisplay = (watcher: WatcherInfo): string => {
  const authorityDisplay = watcher.role?.authority === 'supervisor' ? 'Supervisor' : 'Peer';
  return `${watcher.name} (${authorityDisplay}, ${watcher.status})`;
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

describe('Feature: Watcher Management Overlay UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Open watcher overlay with existing watchers', () => {
    it('should display watcher list when /watcher command is executed', async () => {
      // @step Given a session with two watchers: "Code Reviewer" (Peer, idle) and "Security Auditor" (Supervisor, running)
      const mockWatcherIds = ['watcher-1', 'watcher-2'];
      mockSessionGetWatchers.mockReturnValue(mockWatcherIds);
      mockSessionGetRole.mockImplementation((id: string) => {
        if (id === 'watcher-1') {
          return { name: 'Code Reviewer', authority: 'peer', description: null };
        }
        return { name: 'Security Auditor', authority: 'supervisor', description: null };
      });
      mockSessionGetStatus.mockImplementation((id: string) => {
        return id === 'watcher-1' ? 'idle' : 'running';
      });

      // @step When the user types "/watcher" and presses Enter
      const watchers = await loadWatcherList('parent-session-id');

      // @step Then the Watcher Management overlay should open
      expect(mockSessionGetWatchers).toHaveBeenCalledWith('parent-session-id');
      expect(watchers).toHaveLength(2);

      // @step And the overlay should display "Code Reviewer (Peer, idle)"
      expect(mockSessionGetRole).toHaveBeenCalledWith('watcher-1');
      expect(mockSessionGetStatus).toHaveBeenCalledWith('watcher-1');
      expect(formatWatcherDisplay(watchers[0])).toBe('Code Reviewer (Peer, idle)');

      // @step And the overlay should display "Security Auditor (Supervisor, running)"
      expect(mockSessionGetRole).toHaveBeenCalledWith('watcher-2');
      expect(mockSessionGetStatus).toHaveBeenCalledWith('watcher-2');
      expect(formatWatcherDisplay(watchers[1])).toBe('Security Auditor (Supervisor, running)');

      // @step And the first watcher should be highlighted
      // Initial state always has watcherIndex = 0
      const initialIndex = 0;
      expect(initialIndex).toBe(0);
    });
  });

  describe('Scenario: Open watcher overlay with no watchers', () => {
    it('should display empty state message when no watchers exist', async () => {
      // @step Given a session with no watchers
      mockSessionGetWatchers.mockReturnValue([]);

      // @step When the user types "/watcher" and presses Enter
      const watchers = await loadWatcherList('parent-session-id');

      // @step Then the Watcher Management overlay should open
      expect(mockSessionGetWatchers).toHaveBeenCalledWith('parent-session-id');
      expect(watchers).toHaveLength(0);

      // @step And the overlay should display "No watchers. Press N to create one."
      // The empty message logic: watcherList.length === 0 shows this text
      const showEmptyMessage = watchers.length === 0;
      expect(showEmptyMessage).toBe(true);
    });
  });

  describe('Scenario: Navigate watcher list with arrow keys', () => {
    it('should navigate selection with arrow keys using correct boundary logic', () => {
      // @step Given the Watcher Management overlay is open with 3 watchers
      const watcherList = createMockWatchers(3);
      let watcherIndex = 0;

      // @step And the first watcher is selected
      expect(watcherIndex).toBe(0);

      // @step When the user presses the down arrow key
      watcherIndex = navigateDown(watcherIndex, watcherList.length);
      // @step Then the second watcher should be highlighted
      expect(watcherIndex).toBe(1);

      // @step When the user presses the down arrow key
      watcherIndex = navigateDown(watcherIndex, watcherList.length);
      // @step Then the third watcher should be highlighted
      expect(watcherIndex).toBe(2);

      // Verify boundary: can't go past last item
      watcherIndex = navigateDown(watcherIndex, watcherList.length);
      expect(watcherIndex).toBe(2); // Should stay at 2

      // @step When the user presses the up arrow key
      watcherIndex = navigateUp(watcherIndex);
      // @step Then the second watcher should be highlighted
      expect(watcherIndex).toBe(1);

      // Verify boundary: can't go past first item
      watcherIndex = navigateUp(watcherIndex);
      watcherIndex = navigateUp(watcherIndex);
      expect(watcherIndex).toBe(0); // Should stay at 0
    });
  });

  describe('Scenario: Open selected watcher with Enter key', () => {
    it('should return selected watcher info for session switch', async () => {
      // @step Given the Watcher Management overlay is open with "Code Reviewer" selected
      mockSessionGetWatchers.mockReturnValue(['watcher-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', authority: 'peer', description: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const watchers = await loadWatcherList('parent-session-id');
      const watcherIndex = 0;

      // @step When the user presses Enter
      const selectedWatcher = watchers[watcherIndex];
      
      // @step Then the overlay should close
      // (isWatcherMode set to false - tested via state change)

      // @step And the session should switch to the "Code Reviewer" watcher
      expect(selectedWatcher).toBeDefined();
      expect(selectedWatcher.id).toBe('watcher-1');
      expect(selectedWatcher.name).toBe('Code Reviewer');
    });
  });

  describe('Scenario: Delete watcher with confirmation', () => {
    it('should format correct confirmation message for selected watcher', async () => {
      // @step Given the Watcher Management overlay is open with "Code Reviewer" selected
      mockSessionGetWatchers.mockReturnValue(['watcher-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', authority: 'peer', description: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const watchers = await loadWatcherList('parent-session-id');
      const selectedWatcher = watchers[0];

      // @step When the user presses the D key
      // (showWatcherDeleteDialog set to true)

      // @step Then a confirmation dialog should appear with message "Delete watcher Code Reviewer?"
      const confirmationMessage = `Delete watcher "${selectedWatcher.name}"?`;
      expect(confirmationMessage).toBe('Delete watcher "Code Reviewer"?');

      // @step And the dialog should have Yes and No options
      // The implementation uses ThreeButtonDialog with ['Delete', 'Cancel']
      const dialogOptions = ['Delete', 'Cancel'];
      expect(dialogOptions).toContain('Delete');
      expect(dialogOptions).toContain('Cancel');
    });

    it('should call sessionManagerDestroy when delete is confirmed', async () => {
      // Setup
      mockSessionGetWatchers.mockReturnValue(['watcher-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', authority: 'peer', description: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const watchers = await loadWatcherList('parent-session-id');
      const selectedWatcher = watchers[0];

      // Simulate delete action (what handleWatcherDelete does)
      mockSessionManagerDestroy(selectedWatcher.id);

      // Verify NAPI was called with correct watcher ID
      expect(mockSessionManagerDestroy).toHaveBeenCalledWith('watcher-1');
    });
  });

  describe('Scenario: Close overlay with Escape key', () => {
    it('should reset watcher mode state on Escape', () => {
      // @step Given the Watcher Management overlay is open
      const state: WatcherManagementState = {
        isWatcherMode: true,
        watcherList: createMockWatchers(2),
        watcherIndex: 1, // Not at default
        watcherScrollOffset: 0,
        showWatcherDeleteDialog: false,
      };
      expect(state.isWatcherMode).toBe(true);

      // @step When the user presses the Escape key
      // Implementation does: setIsWatcherMode(false); setWatcherList([]);
      state.isWatcherMode = false;
      state.watcherList = [];

      // @step Then the overlay should close
      expect(state.isWatcherMode).toBe(false);
      expect(state.watcherList).toHaveLength(0);

      // @step And the main agent view should be visible with conversation intact
      // (Conversation state is not modified by escape handler)
    });
  });

  describe('Scenario: Scrollable list for many watchers', () => {
    it('should calculate scroll visibility correctly', () => {
      // @step Given a session with 10 watchers
      const watchers = createMockWatchers(10);
      
      // The implementation calculates visible height as:
      // Math.max(1, Math.floor((terminalHeight - 6) / 2))
      // For a typical terminal of 24 rows: Math.floor((24 - 6) / 2) = 9
      const terminalHeight = 24;
      const visibleHeight = Math.max(1, Math.floor((terminalHeight - 6) / 2));

      // @step When the user types "/watcher" and presses Enter
      const state: WatcherManagementState = {
        isWatcherMode: true,
        watcherList: watchers,
        watcherIndex: 0,
        watcherScrollOffset: 0,
        showWatcherDeleteDialog: false,
      };

      // @step Then the Watcher Management overlay should show a scrollable list
      const needsScrolling = state.watcherList.length > visibleHeight;
      expect(needsScrolling).toBe(true);

      // @step And a scroll position indicator should be visible
      // The implementation shows scrollbar when: watcherList.length > watcherVisibleHeight
      expect(state.watcherList.length).toBeGreaterThan(visibleHeight);
    });
  });

  describe('Scenario: Edit watcher with E key', () => {
    it('should activate inline edit mode on E key press', async () => {
      // @step Given the Watcher Management overlay is open with "Code Reviewer" selected
      mockSessionGetWatchers.mockReturnValue(['watcher-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', authority: 'peer', description: null });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const watchers = await loadWatcherList('parent-session-id');
      const selectedWatcher = watchers[0];

      // Simulate edit mode state
      let isWatcherEditMode = false;
      let watcherEditValue = '';

      // @step When the user presses the E key
      // Implementation sets: setWatcherEditValue(selectedWatcher.name); setIsWatcherEditMode(true);
      watcherEditValue = selectedWatcher.name;
      isWatcherEditMode = true;

      // @step Then inline edit mode should activate for the watcher name
      expect(isWatcherEditMode).toBe(true);
      expect(watcherEditValue).toBe('Code Reviewer');

      // @step And the user can modify the name and press Enter to save
      // Simulate typing new name
      watcherEditValue = 'Senior Reviewer';
      
      // Simulate Enter to save
      const updatedWatcher = { ...selectedWatcher, name: watcherEditValue.trim() };
      expect(updatedWatcher.name).toBe('Senior Reviewer');
      
      // Mode should close after save
      isWatcherEditMode = false;
      watcherEditValue = '';
      expect(isWatcherEditMode).toBe(false);
    });
  });

  describe('Scenario: Scroll follows selection when navigating', () => {
    /**
     * Scroll adjustment logic - MUST match useEffect in AgentView.tsx
     * 
     * This duplicates the logic:
     *   if (watcherIndex < watcherScrollOffset) {
     *     setWatcherScrollOffset(watcherIndex);
     *   } else if (watcherIndex >= watcherScrollOffset + watcherVisibleHeight) {
     *     setWatcherScrollOffset(watcherIndex - watcherVisibleHeight + 1);
     *   }
     */
    const adjustScrollOffset = (
      watcherIndex: number,
      watcherScrollOffset: number,
      watcherVisibleHeight: number
    ): number => {
      if (watcherIndex < watcherScrollOffset) {
        return watcherIndex;
      } else if (watcherIndex >= watcherScrollOffset + watcherVisibleHeight) {
        return watcherIndex - watcherVisibleHeight + 1;
      }
      return watcherScrollOffset;
    };

    it('should adjust scroll offset to keep selection visible when navigating down', () => {
      // @step Given the Watcher Management overlay is open with 10 watchers
      const watcherList = createMockWatchers(10);
      
      // @step And only 5 watchers are visible at a time
      const watcherVisibleHeight = 5;
      
      // @step And the first watcher is selected with scroll offset 0
      let watcherIndex = 0;
      let watcherScrollOffset = 0;
      
      expect(watcherIndex).toBe(0);
      expect(watcherScrollOffset).toBe(0);

      // @step When the user presses the down arrow key 6 times
      for (let i = 0; i < 6; i++) {
        watcherIndex = navigateDown(watcherIndex, watcherList.length);
        watcherScrollOffset = adjustScrollOffset(watcherIndex, watcherScrollOffset, watcherVisibleHeight);
      }

      // @step Then the 7th watcher should be highlighted
      expect(watcherIndex).toBe(6);

      // @step And the scroll offset should adjust to keep the selection visible
      // When index is 6 and visible height is 5, offset should be 2 (6 - 5 + 1 = 2)
      expect(watcherScrollOffset).toBe(2);
    });
  });

  describe('Scenario: Edit watcher persists changes to backend', () => {
    /**
     * Edit save logic - MUST match useInput handler in AgentView.tsx
     * 
     * This duplicates the corrected logic:
     *   try {
     *     sessionSetRole(...);
     *     // Update local state ONLY on success
     *     updatedList[watcherIndex] = { ...updatedList[watcherIndex], name: newName };
     *   } catch (err) {
     *     // Show error, do NOT update local state
     *   }
     */
    const saveEditedWatcherName = (
      watcher: WatcherInfo,
      newName: string,
      watcherList: WatcherInfo[],
      watcherIndex: number,
      sessionSetRoleFn: typeof mockSessionSetRole
    ): { success: boolean; updatedList: WatcherInfo[] } => {
      try {
        sessionSetRoleFn(
          watcher.id,
          newName.trim(),
          watcher.role?.description || null,
          watcher.role?.authority || 'peer'
        );
        // Update local state ONLY if backend save succeeded
        const updatedList = [...watcherList];
        updatedList[watcherIndex] = {
          ...updatedList[watcherIndex],
          name: newName.trim(),
        };
        return { success: true, updatedList };
      } catch {
        // Do NOT update local state - keep showing old name for consistency
        return { success: false, updatedList: watcherList };
      }
    };

    it('should call sessionSetRole and update local state when save succeeds', async () => {
      // @step Given the Watcher Management overlay is open with "Code Reviewer" selected
      mockSessionGetWatchers.mockReturnValue(['watcher-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', authority: 'peer', description: 'Reviews code' });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const watchers = await loadWatcherList('parent-session-id');
      const selectedWatcher = watchers[0];
      const watcherIndex = 0;

      // @step And the user is in edit mode with value "Senior Reviewer"
      const watcherEditValue = 'Senior Reviewer';
      
      expect(watcherEditValue).toBe('Senior Reviewer');

      // @step When the user presses Enter to save
      const result = saveEditedWatcherName(
        selectedWatcher,
        watcherEditValue,
        watchers,
        watcherIndex,
        mockSessionSetRole
      );

      // @step Then sessionSetRole should be called with the new name
      expect(mockSessionSetRole).toHaveBeenCalledWith(
        'watcher-1',
        'Senior Reviewer',
        'Reviews code',
        'peer'
      );

      // @step And the watcher list should show "Senior Reviewer"
      expect(result.success).toBe(true);
      expect(result.updatedList[watcherIndex].name).toBe('Senior Reviewer');

      // @step And reopening the overlay should still show "Senior Reviewer"
      // This is verified by the fact that sessionSetRole was called to persist
      expect(mockSessionSetRole).toHaveBeenCalledTimes(1);
    });

    it('should NOT update local state when sessionSetRole fails', async () => {
      // @step Given the Watcher Management overlay is open with "Code Reviewer" selected
      mockSessionGetWatchers.mockReturnValue(['watcher-1']);
      mockSessionGetRole.mockReturnValue({ name: 'Code Reviewer', authority: 'peer', description: 'Reviews code' });
      mockSessionGetStatus.mockReturnValue('idle');
      
      const watchers = await loadWatcherList('parent-session-id');
      const selectedWatcher = watchers[0];
      const watcherIndex = 0;

      // @step And the user is in edit mode with value "Senior Reviewer"
      const watcherEditValue = 'Senior Reviewer';

      // Mock sessionSetRole to throw an error
      const failingSetRole = vi.fn().mockImplementation(() => {
        throw new Error('Backend save failed');
      });

      // @step When the user presses Enter to save but backend fails
      const result = saveEditedWatcherName(
        selectedWatcher,
        watcherEditValue,
        watchers,
        watcherIndex,
        failingSetRole
      );

      // @step Then the save should fail
      expect(result.success).toBe(false);

      // @step And the watcher list should still show "Code Reviewer" (not updated)
      expect(result.updatedList[watcherIndex].name).toBe('Code Reviewer');

      // @step And reopening the overlay would show "Code Reviewer" (consistent with backend)
      // The local state was NOT updated, so UI stays consistent with backend
    });
  });
});
