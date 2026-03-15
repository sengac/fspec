/**
 * Feature: spec/features/boardview-loaddata-stability.feature
 *
 * BUG-119: BoardView flickering caused by TUI-079 loadData() change —
 * lock contention triggers error-state oscillation.
 *
 * Tests verify that:
 * - Lock contention errors are treated as transient (no ErrorView flash)
 * - loadData() has an in-flight guard preventing concurrent calls
 * - globalStreamListener debounces WorkUnitsUpdate events
 * - Error state is only cleared on successful reads
 * - BoardView does not call loadData() explicitly after move operations
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useFspecStore, _resetLoadDataGuard } from '../fspecStore';
import { useSessionStore } from '../sessionStore';

// Mock codelet-napi
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetActive: vi.fn(() => null),
  sessionGetWorkUnitContext: vi.fn(() => null),
  startWorkUnitsWatcher: vi.fn(),
  stopWorkUnitsWatcher: vi.fn(),
  isWorkUnitsWatcherActive: vi.fn(() => false),
  sessionSetWorkUnitContext: vi.fn(),
}));

// Mock ensure-files so loadData doesn't hit disk
vi.mock('../../../utils/ensure-files', () => ({
  ensureWorkUnitsFile: vi.fn(),
  ensureEpicsFile: vi.fn(() => ({ epics: {} })),
}));

// Mock logger
vi.mock('../../../utils/logger', () => ({
  logger: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

import {
  initGlobalStreamListener,
  stopGlobalStreamListener,
} from '../globalStreamListener';
import {
  startWorkUnitsWatcher,
  isWorkUnitsWatcherActive,
  sessionGetActive,
  sessionSetWorkUnitContext,
} from '@sengac/codelet-napi';
import {
  ensureWorkUnitsFile,
  ensureEpicsFile,
} from '../../../utils/ensure-files';
import { logger } from '../../../utils/logger';

// Helper: capture watcher callback from init
async function initAndCaptureCallback(): Promise<
  (chunk: Record<string, unknown>) => void
> {
  const mockStartWatcher = vi.mocked(startWorkUnitsWatcher);
  mockStartWatcher.mockClear();
  await initGlobalStreamListener('/test/project');
  const callback = mockStartWatcher.mock.calls[0]?.[1];
  if (!callback) {
    throw new Error('startWorkUnitsWatcher was not called with a callback');
  }
  return callback as (chunk: Record<string, unknown>) => void;
}

// Helper: build standard work units file data
function makeWorkUnitsData(
  units: Array<{ id: string; title: string; status: string }>
) {
  const workUnits: Record<string, unknown> = {};
  const states: Record<string, string[]> = {
    backlog: [],
    specifying: [],
    testing: [],
    implementing: [],
    validating: [],
    done: [],
    blocked: [],
  };
  for (const u of units) {
    workUnits[u.id] = {
      id: u.id,
      title: u.title,
      status: u.status,
      type: 'story',
    };
    if (states[u.status]) {
      states[u.status].push(u.id);
    }
  }
  return { workUnits, states };
}

describe('Feature: BoardView loadData stability (BUG-119)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.mocked(isWorkUnitsWatcherActive).mockReturnValue(false);
    vi.mocked(sessionGetActive).mockReturnValue(null);

    // BUG-119: Reset module-level in-flight guard to prevent leakage across tests
    _resetLoadDataGuard();

    // Reset stores to clean state
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
      cwd: '/test/project',
      checkpointCounts: { manual: 0, auto: 0 },
      sessionAttachments: new Map(),
    });

    useSessionStore.setState({
      currentWorkUnitId: null,
      currentWorkUnitStatus: null,
    });
  });

  afterEach(() => {
    stopGlobalStreamListener();
    vi.useRealTimers();
  });

  describe('Scenario: Lock contention error does not trigger ErrorView', () => {
    it('should keep error state null and preserve existing work units on lock contention', async () => {
      // @step Given the fspec store has loaded work units successfully
      const initialData = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
        { id: 'WU-002', title: 'Task 2', status: 'implementing' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValueOnce(initialData);
      vi.mocked(ensureEpicsFile).mockResolvedValueOnce({ epics: {} });
      await useFspecStore.getState().loadData();

      expect(useFspecStore.getState().workUnits).toHaveLength(2);
      expect(useFspecStore.getState().error).toBeNull();

      // @step When loadData() encounters a "Lock file is already being held" error
      const lockError = new Error('Lock file is already being held');
      vi.mocked(ensureWorkUnitsFile).mockRejectedValueOnce(lockError);
      await useFspecStore.getState().loadData();

      // @step Then the store error state must remain null
      expect(useFspecStore.getState().error).toBeNull();

      // @step And the existing work units must remain unchanged in the store
      expect(useFspecStore.getState().workUnits).toHaveLength(2);
      expect(useFspecStore.getState().workUnits[0].id).toBe('WU-001');

      // @step And the error is logged at debug level as transient
      expect(vi.mocked(logger.debug)).toHaveBeenCalledWith(
        expect.stringContaining('lock contention')
      );
    });
  });

  describe('Scenario: Lock contention does not clear prior successful state', () => {
    it('should preserve work units and isLoaded on lock contention', async () => {
      // @step Given the fspec store has loaded work units successfully
      // @step And the store has 5 work units displayed
      const initialData = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
        { id: 'WU-002', title: 'Task 2', status: 'backlog' },
        { id: 'WU-003', title: 'Task 3', status: 'implementing' },
        { id: 'WU-004', title: 'Task 4', status: 'testing' },
        { id: 'WU-005', title: 'Task 5', status: 'done' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValueOnce(initialData);
      vi.mocked(ensureEpicsFile).mockResolvedValueOnce({ epics: {} });
      await useFspecStore.getState().loadData();

      // @step When loadData() is called and encounters a lock contention error
      vi.mocked(ensureWorkUnitsFile).mockRejectedValueOnce(
        new Error('Lock file is already being held')
      );
      await useFspecStore.getState().loadData();

      // @step Then the store must still have 5 work units
      expect(useFspecStore.getState().workUnits).toHaveLength(5);

      // @step And isLoaded must remain true
      expect(useFspecStore.getState().isLoaded).toBe(true);
    });
  });

  describe('Scenario: Real errors still set error state', () => {
    it('should set error state for permission denied errors', async () => {
      // @step Given the fspec store has loaded work units successfully
      const initialData = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValueOnce(initialData);
      vi.mocked(ensureEpicsFile).mockResolvedValueOnce({ epics: {} });
      await useFspecStore.getState().loadData();

      // @step When loadData() encounters a permission denied error
      const permError = new Error('EACCES: permission denied');
      vi.mocked(ensureWorkUnitsFile).mockRejectedValueOnce(permError);
      await useFspecStore.getState().loadData();

      // @step Then the store error state must be set with the error details
      expect(useFspecStore.getState().error).toBeTruthy();
      expect(useFspecStore.getState().error).toContain('permission denied');

      // @step And the ErrorView should be displayed
      // (verified by error state being non-null — BoardView renders ErrorView when error is truthy)
    });
  });

  describe('Scenario: loadData clears error only on success not before reading', () => {
    it('should not clear error before read attempt', async () => {
      // @step Given the fspec store has a previous error state set
      useFspecStore.setState({ error: 'Previous error occurred' });
      expect(useFspecStore.getState().error).toBe('Previous error occurred');

      // Track state changes to verify order
      const stateChanges: Array<{ error: string | null }> = [];
      const unsub = useFspecStore.subscribe(state => {
        stateChanges.push({ error: state.error });
      });

      // @step When loadData() is called and succeeds
      const successData = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValueOnce(successData);
      vi.mocked(ensureEpicsFile).mockResolvedValueOnce({ epics: {} });
      await useFspecStore.getState().loadData();

      unsub();

      // @step Then the error state must be cleared after the read completes
      expect(useFspecStore.getState().error).toBeNull();

      // @step And the error state must not be cleared before attempting the read
      // The error should be cleared in a single state update with workUnits (not a separate pre-clear)
      // With the fix, there should be exactly ONE state change that sets error to null + workUnits
      const errorClears = stateChanges.filter(s => s.error === null);
      expect(errorClears.length).toBe(1);
    });
  });

  describe('Scenario: Concurrent loadData calls are prevented by in-flight guard', () => {
    it('should skip second loadData when one is already running', async () => {
      // @step Given loadData() is already in-flight
      // Create a slow promise that we control
      let resolveFirst: ((value: unknown) => void) | undefined;
      const slowPromise = new Promise(resolve => {
        resolveFirst = resolve;
      });
      vi.mocked(ensureWorkUnitsFile).mockReturnValueOnce(slowPromise as never);

      // Start first loadData (it will block on ensureWorkUnitsFile)
      const firstLoad = useFspecStore.getState().loadData();

      // @step When another loadData() call is triggered
      const callCountBefore = vi.mocked(ensureWorkUnitsFile).mock.calls.length;
      const secondLoad = useFspecStore.getState().loadData();
      await secondLoad; // second should return immediately

      // @step Then the second call must return immediately without acquiring any locks
      const callCountAfter = vi.mocked(ensureWorkUnitsFile).mock.calls.length;
      expect(callCountAfter).toBe(callCountBefore); // No additional call

      // @step And only one loadData execution is running at any time
      // Resolve the first load so cleanup happens
      const data = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
      ]);
      resolveFirst!(data);
      vi.mocked(ensureEpicsFile).mockResolvedValueOnce({ epics: {} });
      await firstLoad;
    });
  });

  describe('Scenario: globalStreamListener debounces WorkUnitsUpdate events', () => {
    it('should call loadData exactly once for multiple rapid events', async () => {
      // @step Given the globalStreamListener is initialized
      const callback = await initAndCaptureCallback();
      const successData = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue(successData);
      vi.mocked(ensureEpicsFile).mockResolvedValue({ epics: {} });

      // Clear any calls from init
      vi.mocked(ensureWorkUnitsFile).mockClear();

      // @step When 3 WorkUnitsUpdate events arrive within 150ms
      callback({ type: 'WorkUnitsUpdate', workUnits: [] });
      await vi.advanceTimersByTimeAsync(50);
      callback({ type: 'WorkUnitsUpdate', workUnits: [] });
      await vi.advanceTimersByTimeAsync(50);
      callback({ type: 'WorkUnitsUpdate', workUnits: [] });

      // @step Then loadData() must be called exactly once
      // @step And the call must happen after the debounce period elapses
      // Before debounce period: no calls
      expect(vi.mocked(ensureWorkUnitsFile)).not.toHaveBeenCalled();

      // Advance past debounce period
      await vi.advanceTimersByTimeAsync(300);

      // Now exactly one call should have been made
      expect(vi.mocked(ensureWorkUnitsFile)).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Debounce timer resets on each new event', () => {
    it('should reset debounce timer when new event arrives', async () => {
      // @step Given the globalStreamListener is initialized
      const callback = await initAndCaptureCallback();
      const successData = makeWorkUnitsData([
        { id: 'WU-001', title: 'Task 1', status: 'backlog' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue(successData);
      vi.mocked(ensureEpicsFile).mockResolvedValue({ epics: {} });
      vi.mocked(ensureWorkUnitsFile).mockClear();

      // @step And a WorkUnitsUpdate event arrived 50ms ago
      callback({ type: 'WorkUnitsUpdate', workUnits: [] });
      await vi.advanceTimersByTimeAsync(50);

      // @step When another WorkUnitsUpdate event arrives
      callback({ type: 'WorkUnitsUpdate', workUnits: [] });

      // @step Then the debounce timer must reset
      // Advance 100ms (would be past original debounce if not reset)
      await vi.advanceTimersByTimeAsync(100);
      expect(vi.mocked(ensureWorkUnitsFile)).not.toHaveBeenCalled();

      // @step And loadData() must not be called until the debounce period elapses from the latest event
      await vi.advanceTimersByTimeAsync(200);
      expect(vi.mocked(ensureWorkUnitsFile)).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Session context sync preserved after debounced loadData', () => {
    it('should sync session status after debounced reload', async () => {
      // Use real timers for this test since we need async promise resolution
      // to work naturally. We test the session sync logic, not the debounce timing.
      vi.useRealTimers();

      // @step Given a session is attached to work unit AUTH-001
      useSessionStore.setState({
        currentWorkUnitId: 'AUTH-001',
        currentWorkUnitStatus: 'backlog',
      });
      vi.mocked(sessionGetActive).mockReturnValue('session-123');

      const callback = await initAndCaptureCallback();

      // @step And AUTH-001 status changes externally from backlog to implementing
      const updatedData = makeWorkUnitsData([
        { id: 'AUTH-001', title: 'Auth Task', status: 'implementing' },
      ]);
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue(updatedData);
      vi.mocked(ensureEpicsFile).mockResolvedValue({ epics: {} });

      // @step When the debounced loadData completes
      callback({ type: 'WorkUnitsUpdate', workUnits: [] });

      // Wait for debounce (150ms) + loadData promise + .then chain + dynamic import
      await new Promise(resolve => {
        setTimeout(resolve, 400);
      });

      // @step Then the session store must update currentWorkUnitStatus to implementing
      expect(useSessionStore.getState().currentWorkUnitStatus).toBe(
        'implementing'
      );

      // @step And Rust context must be updated with the new status
      expect(vi.mocked(sessionSetWorkUnitContext)).toHaveBeenCalledWith(
        'session-123',
        'AUTH-001',
        'Auth Task',
        'implementing'
      );
    });
  });
});
