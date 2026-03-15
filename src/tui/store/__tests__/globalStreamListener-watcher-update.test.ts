/**
 * Feature: spec/features/board-view-watcher-update.feature
 *
 * TUI-079: BoardView does not fully update when work-units.json changes —
 * globalStreamListener uses lossy updateWorkUnitsFromWatcher path.
 *
 * Tests verify that globalStreamListener calls loadData() (full re-read)
 * instead of updateWorkUnitsFromWatcher() (partial patch), and that session
 * context is properly synced from store data after reload.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useFspecStore } from '../fspecStore';
import { useSessionStore } from '../sessionStore';

// Mock codelet-napi before importing globalStreamListener
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

// Mock logger to silence output
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
import { ensureWorkUnitsFile } from '../../../utils/ensure-files';

// Helper to capture the watcher callback registered during init
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

// Helper to build a WorkUnitsUpdate chunk
function makeWorkUnitsUpdateChunk(
  workUnits: Array<{
    id: string;
    title: string;
    workType: string;
    status: string;
    description?: string;
    estimate?: number;
    epic?: string;
  }>
): Record<string, unknown> {
  return { type: 'WorkUnitsUpdate', workUnits };
}

describe('Feature: BoardView watcher update (TUI-079)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(isWorkUnitsWatcherActive).mockReturnValue(false);
    vi.mocked(sessionGetActive).mockReturnValue(null);

    // Reset stores
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
      cwd: '/test/project',
      sessionAttachments: new Map<string, string>(),
    });

    useSessionStore.setState({
      currentWorkUnitId: null,
      currentWorkUnitStatus: null,
    });

    // Reset globalStreamListener internal state
    stopGlobalStreamListener();
  });

  afterEach(() => {
    stopGlobalStreamListener();
  });

  // ========================================
  // GAP 1 + GAP 5: Ordering from states arrays
  // ========================================

  describe('Scenario: Work unit status change preserves correct column priority order', () => {
    it('should order work units according to states arrays after watcher reload', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Two',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And the backlog column shows "AUTH-001" at position 1 and "AUTH-002" at position 2
      let state = useFspecStore.getState();
      const backlogBefore = state.workUnits.filter(
        wu => wu.status === 'backlog'
      );
      expect(backlogBefore[0].id).toBe('AUTH-001');
      expect(backlogBefore[1].id).toBe('AUTH-002');

      // @step When an external process moves "AUTH-001" from backlog to specifying
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'specifying',
            type: 'story',
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Two',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-002'],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'Auth One',
            workType: 'story',
            status: 'specifying',
          },
          {
            id: 'AUTH-002',
            title: 'Auth Two',
            workType: 'story',
            status: 'backlog',
          },
        ])
      );

      // Allow async loadData to complete
      await vi.waitFor(() => {
        state = useFspecStore.getState();
        expect(
          state.workUnits.some(
            wu => wu.id === 'AUTH-001' && wu.status === 'specifying'
          )
        ).toBe(true);
      });

      // @step Then the globalStreamListener should call loadData instead of updateWorkUnitsFromWatcher
      expect(ensureWorkUnitsFile).toHaveBeenCalled();

      // @step And "AUTH-001" should appear in the specifying column
      state = useFspecStore.getState();
      const specifying = state.workUnits.filter(
        wu => wu.status === 'specifying'
      );
      expect(specifying.map(wu => wu.id)).toContain('AUTH-001');

      // @step And "AUTH-001" should be in the position defined by the states.specifying array
      expect(specifying[0].id).toBe('AUTH-001');
    });
  });

  describe('Scenario: External priority reordering is reflected on the board', () => {
    it('should reflect reordered states arrays on the board', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Two',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And the backlog column shows "AUTH-001" at position 1 and "AUTH-002" at position 2
      let backlog = useFspecStore
        .getState()
        .workUnits.filter(wu => wu.status === 'backlog');
      expect(backlog[0].id).toBe('AUTH-001');
      expect(backlog[1].id).toBe('AUTH-002');

      // @step When an external process reorders the states.backlog array to ["AUTH-002", "AUTH-001"]
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Two',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-002', 'AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'Auth One',
            workType: 'story',
            status: 'backlog',
          },
          {
            id: 'AUTH-002',
            title: 'Auth Two',
            workType: 'story',
            status: 'backlog',
          },
        ])
      );

      await vi.waitFor(() => {
        backlog = useFspecStore
          .getState()
          .workUnits.filter(wu => wu.status === 'backlog');
        // @step Then the backlog column should show "AUTH-002" at position 1 and "AUTH-001" at position 2
        expect(backlog[0].id).toBe('AUTH-002');
        expect(backlog[1].id).toBe('AUTH-001');
      });
    });
  });

  // ========================================
  // GAP 2: stateHistory for last-changed indicator
  // ========================================

  describe('Scenario: Last-changed indicator updates when status changes externally', () => {
    it('should load fresh stateHistory so last-changed indicator moves', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
            stateHistory: [
              { state: 'backlog', timestamp: '2026-03-01T00:00:00Z' },
            ],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Two',
            status: 'backlog',
            type: 'story',
            stateHistory: [
              { state: 'backlog', timestamp: '2026-03-10T00:00:00Z' },
            ],
          },
        },
        states: {
          backlog: ['AUTH-001', 'AUTH-002'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And the last-changed indicator is showing on "AUTH-002"
      let wus = useFspecStore.getState().workUnits;
      const lastChangedBefore = wus.reduce((latest, current) => {
        const latestTs = latest.stateHistory?.length
          ? new Date(
              latest.stateHistory[latest.stateHistory.length - 1].timestamp
            ).getTime()
          : 0;
        const currentTs = current.stateHistory?.length
          ? new Date(
              current.stateHistory[current.stateHistory.length - 1].timestamp
            ).getTime()
          : 0;
        return currentTs > latestTs ? current : latest;
      });
      expect(lastChangedBefore.id).toBe('AUTH-002');

      // @step When an external process changes the status of "AUTH-001" to specifying
      // @step And "AUTH-001" now has the most recent stateHistory timestamp
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'specifying',
            type: 'story',
            stateHistory: [
              { state: 'backlog', timestamp: '2026-03-01T00:00:00Z' },
              { state: 'specifying', timestamp: '2026-03-15T12:00:00Z' },
            ],
          },
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Auth Two',
            status: 'backlog',
            type: 'story',
            stateHistory: [
              { state: 'backlog', timestamp: '2026-03-10T00:00:00Z' },
            ],
          },
        },
        states: {
          backlog: ['AUTH-002'],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'Auth One',
            workType: 'story',
            status: 'specifying',
          },
          {
            id: 'AUTH-002',
            title: 'Auth Two',
            workType: 'story',
            status: 'backlog',
          },
        ])
      );

      // @step Then the last-changed indicator should move to "AUTH-001"
      await vi.waitFor(() => {
        wus = useFspecStore.getState().workUnits;
        const lastChanged = wus.reduce((latest, current) => {
          const latestTs = latest.stateHistory?.length
            ? new Date(
                latest.stateHistory[latest.stateHistory.length - 1].timestamp
              ).getTime()
            : 0;
          const currentTs = current.stateHistory?.length
            ? new Date(
                current.stateHistory[current.stateHistory.length - 1].timestamp
              ).getTime()
            : 0;
          return currentTs > latestTs ? current : latest;
        });
        expect(lastChanged.id).toBe('AUTH-001');
      });
    });
  });

  // ========================================
  // GAP 3: Attachments visible after external change
  // ========================================

  describe('Scenario: Attachment added externally appears in details panel', () => {
    it('should load fresh attachments after watcher-triggered reload', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'TOOL-014': {
            id: 'TOOL-014',
            title: 'Tool Fourteen',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['TOOL-014'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And work unit "TOOL-014" has no attachments
      let wu = useFspecStore
        .getState()
        .workUnits.find(w => w.id === 'TOOL-014');
      expect(wu?.attachments).toBeUndefined();

      // @step When an external process adds an attachment to "TOOL-014"
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'TOOL-014': {
            id: 'TOOL-014',
            title: 'Tool Fourteen',
            status: 'backlog',
            type: 'story',
            attachments: ['spec/attachments/TOOL-014/diagram.png'],
          },
        },
        states: {
          backlog: ['TOOL-014'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'TOOL-014',
            title: 'Tool Fourteen',
            workType: 'story',
            status: 'backlog',
          },
        ])
      );

      // @step Then the details panel for "TOOL-014" should show the attachment
      await vi.waitFor(() => {
        wu = useFspecStore.getState().workUnits.find(w => w.id === 'TOOL-014');
        expect(wu?.attachments).toEqual([
          'spec/attachments/TOOL-014/diagram.png',
        ]);
      });
    });
  });

  // ========================================
  // GAP 4: Deleted work units removed
  // ========================================

  describe('Scenario: Deleted work unit disappears from the board', () => {
    it('should remove deleted work units after watcher-triggered reload', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-003': {
            id: 'AUTH-003',
            title: 'Auth Three',
            status: 'backlog',
            type: 'story',
          },
          'AUTH-004': {
            id: 'AUTH-004',
            title: 'Auth Four',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-003', 'AUTH-004'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And "AUTH-003" is visible in the backlog column
      expect(
        useFspecStore.getState().workUnits.find(wu => wu.id === 'AUTH-003')
      ).toBeDefined();

      // @step When an external process deletes "AUTH-003" from work-units.json
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-004': {
            id: 'AUTH-004',
            title: 'Auth Four',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-004'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-004',
            title: 'Auth Four',
            workType: 'story',
            status: 'backlog',
          },
        ])
      );

      // @step Then "AUTH-003" should no longer appear on the board
      await vi.waitFor(() => {
        const wus = useFspecStore.getState().workUnits;
        expect(wus.find(wu => wu.id === 'AUTH-003')).toBeUndefined();
        expect(wus).toHaveLength(1);
        expect(wus[0].id).toBe('AUTH-004');
      });
    });
  });

  // ========================================
  // GAP 6: loadData called instead of updateWorkUnitsFromWatcher
  // ========================================

  describe('Scenario: globalStreamListener calls loadData on WorkUnitsUpdate event', () => {
    it('should call loadData and NOT updateWorkUnitsFromWatcher', async () => {
      // @step Given the globalStreamListener is initialized
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();
      const callCountAfterInit =
        vi.mocked(ensureWorkUnitsFile).mock.calls.length;

      // Set up a different response for the reload (proves loadData was called, not updateWorkUnitsFromWatcher)
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'RELOADED',
            status: 'specifying',
            type: 'story',
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step When a WorkUnitsUpdate stream chunk is received
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'CHUNK TITLE',
            workType: 'story',
            status: 'specifying',
          },
        ])
      );

      // @step Then loadData should be called on fspecStore
      await vi.waitFor(() => {
        expect(
          vi.mocked(ensureWorkUnitsFile).mock.calls.length
        ).toBeGreaterThan(callCountAfterInit);
      });

      // @step And updateWorkUnitsFromWatcher should NOT be called
      // Proof: title comes from the file mock ("RELOADED"), not the chunk ("CHUNK TITLE")
      await vi.waitFor(() => {
        const wu = useFspecStore
          .getState()
          .workUnits.find(w => w.id === 'AUTH-001');
        expect(wu?.title).toBe('RELOADED');
        expect(wu?.title).not.toBe('CHUNK TITLE');
      });
    });
  });

  // ========================================
  // GAP 7: Watcher event used only as signal
  // ========================================

  describe('Scenario: Watcher event chunk data is not used for store updates', () => {
    it('should use loadData file re-read, not chunk.workUnits partial data', async () => {
      // @step Given the globalStreamListener is initialized
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
            stateHistory: [
              { state: 'backlog', timestamp: '2026-03-01T00:00:00Z' },
            ],
            attachments: ['spec/attachments/AUTH-001/notes.md'],
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // Now set up a different file read for the reload
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One Updated',
            status: 'specifying',
            type: 'story',
            stateHistory: [
              { state: 'backlog', timestamp: '2026-03-01T00:00:00Z' },
              { state: 'specifying', timestamp: '2026-03-15T00:00:00Z' },
            ],
            attachments: [
              'spec/attachments/AUTH-001/notes.md',
              'spec/attachments/AUTH-001/diagram.png',
            ],
          },
        },
        states: {
          backlog: [],
          specifying: ['AUTH-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step When a WorkUnitsUpdate stream chunk is received with partial work unit data
      // The chunk only has 7 fields — no stateHistory or attachments
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'Chunk Title (partial)',
            workType: 'story',
            status: 'specifying',
          },
        ])
      );

      // @step Then the store should be updated from the full file re-read via loadData
      await vi.waitFor(() => {
        const wu = useFspecStore
          .getState()
          .workUnits.find(w => w.id === 'AUTH-001');
        // Title should come from the file, not the chunk
        expect(wu?.title).toBe('Auth One Updated');
      });

      // @step And the chunk.workUnits data should not be passed to any store update function
      const wu = useFspecStore
        .getState()
        .workUnits.find(w => w.id === 'AUTH-001');
      // Verify full fields are present (only possible via loadData, not chunk)
      expect(wu?.stateHistory).toHaveLength(2);
      expect(wu?.attachments).toHaveLength(2);
      // The chunk had "Chunk Title (partial)" — if that's the title, chunk data was used
      expect(wu?.title).not.toBe('Chunk Title (partial)');
    });
  });

  // ========================================
  // GAP 8: Session context cleared for deleted work unit
  // ========================================

  describe('Scenario: Session context cleared when attached work unit is deleted externally', () => {
    it('should clear session context when attached work unit is deleted', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'TOOL-014': {
            id: 'TOOL-014',
            title: 'Tool Fourteen',
            status: 'backlog',
            type: 'story',
          },
          'TOOL-015': {
            id: 'TOOL-015',
            title: 'Tool Fifteen',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['TOOL-014', 'TOOL-015'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And a session is attached to work unit "TOOL-014"
      useFspecStore.getState().attachSession('TOOL-014', 'session-abc');
      vi.mocked(sessionGetActive).mockReturnValue('session-abc');

      // @step And sessionStore.currentWorkUnitId is "TOOL-014"
      useSessionStore.getState().setCurrentWorkUnit('TOOL-014', 'backlog');
      expect(useSessionStore.getState().currentWorkUnitId).toBe('TOOL-014');

      // @step When an external process deletes "TOOL-014" from work-units.json
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'TOOL-015': {
            id: 'TOOL-015',
            title: 'Tool Fifteen',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['TOOL-015'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'TOOL-015',
            title: 'Tool Fifteen',
            workType: 'story',
            status: 'backlog',
          },
        ])
      );

      // @step Then "TOOL-014" should no longer appear on the board
      await vi.waitFor(() => {
        expect(
          useFspecStore.getState().workUnits.find(wu => wu.id === 'TOOL-014')
        ).toBeUndefined();
      });

      // @step And sessionStore.currentWorkUnitId should be null
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();

      // @step And sessionStore.currentWorkUnitStatus should be null
      expect(useSessionStore.getState().currentWorkUnitStatus).toBeNull();

      // Verify Rust-side context is also cleared to prevent stale reattachment
      expect(vi.mocked(sessionSetWorkUnitContext)).toHaveBeenCalledWith(
        'session-abc',
        null,
        null,
        null
      );
    });
  });

  // ========================================
  // Session header status sync still works
  // ========================================

  describe('Scenario: Session header status syncs from store data after watcher reload', () => {
    it('should update session status from store data, not chunk data', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And a session is attached to work unit "AUTH-001"
      vi.mocked(sessionGetActive).mockReturnValue('session-xyz');

      // @step And sessionStore.currentWorkUnitStatus is "backlog"
      useSessionStore.getState().setCurrentWorkUnit('AUTH-001', 'backlog');
      expect(useSessionStore.getState().currentWorkUnitStatus).toBe('backlog');

      // @step When an external process changes "AUTH-001" status to "implementing"
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'implementing',
            type: 'story',
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['AUTH-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'Auth One',
            workType: 'story',
            status: 'implementing',
          },
        ])
      );

      // @step Then sessionStore.currentWorkUnitStatus should be "implementing"
      await vi.waitFor(() => {
        expect(useSessionStore.getState().currentWorkUnitStatus).toBe(
          'implementing'
        );
      });

      // @step And the status should be read from the store's reloaded data not from chunk.workUnits
      // Verified implicitly: ensureWorkUnitsFile was called (loadData path)
      // The status 'implementing' matches the file data, confirming store-based sync
      expect(ensureWorkUnitsFile).toHaveBeenCalledTimes(2);
    });
  });

  // ========================================
  // New work unit appears correctly
  // ========================================

  describe('Scenario: New work unit created externally appears on the board', () => {
    it('should show new work unit in correct column and position', async () => {
      // @step Given the TUI board is open with work units loaded
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
        },
        states: {
          backlog: ['AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });
      await useFspecStore.getState().loadData();

      // @step And "INFRA-001" does not exist on the board
      expect(
        useFspecStore.getState().workUnits.find(wu => wu.id === 'INFRA-001')
      ).toBeUndefined();

      // @step When an external process creates "INFRA-001" with status "backlog"
      vi.mocked(ensureWorkUnitsFile).mockResolvedValue({
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Auth One',
            status: 'backlog',
            type: 'story',
          },
          'INFRA-001': {
            id: 'INFRA-001',
            title: 'Infra One',
            status: 'backlog',
            type: 'task',
          },
        },
        states: {
          backlog: ['INFRA-001', 'AUTH-001'],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      });

      // @step And the file watcher triggers a WorkUnitsUpdate event
      const callback = await initAndCaptureCallback();
      callback(
        makeWorkUnitsUpdateChunk([
          {
            id: 'AUTH-001',
            title: 'Auth One',
            workType: 'story',
            status: 'backlog',
          },
          {
            id: 'INFRA-001',
            title: 'Infra One',
            workType: 'task',
            status: 'backlog',
          },
        ])
      );

      // @step Then "INFRA-001" should appear in the backlog column
      await vi.waitFor(() => {
        const wus = useFspecStore.getState().workUnits;
        expect(wus.find(wu => wu.id === 'INFRA-001')).toBeDefined();
      });

      // @step And "INFRA-001" should be in the position defined by the states.backlog array
      const backlog = useFspecStore
        .getState()
        .workUnits.filter(wu => wu.status === 'backlog');
      expect(backlog[0].id).toBe('INFRA-001');
      expect(backlog[1].id).toBe('AUTH-001');
    });
  });
});
