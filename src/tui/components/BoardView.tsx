/**
 * BoardView - Interactive Kanban Board Component
 *
 * Coverage:
 * - BOARD-002: Interactive Kanban board CLI
 * - BOARD-003: Real-time board updates with git stash and file inspection
 * - ITF-004: Fix TUI Kanban column layout to match table style
 */

import React, { useEffect, useState, useRef } from 'react';
import { Box, Text, useInput } from 'ink';
import { useFspecStore } from '../store/fspecStore';
import git from 'isomorphic-git';
import fs from 'fs';
import path from 'path';
import chokidar from 'chokidar';
import { getStagedFiles, getUnstagedFiles } from '../../git/status';
import { UnifiedBoardLayout } from './UnifiedBoardLayout';
import { FullScreenWrapper } from './FullScreenWrapper';
import { VirtualList } from './VirtualList';
import { CheckpointViewer } from './CheckpointViewer';
import { ChangedFilesViewer } from './ChangedFilesViewer';
import { useStdout } from 'ink';
import { createIPCServer, cleanupIPCServer, getIPCPath } from '../../utils/ipc';
import type { Server } from 'net';

interface BoardViewProps {
  onExit?: () => void;
  showStashPanel?: boolean;
  showFilesPanel?: boolean;
  focusedPanel?: 'board' | 'stash' | 'files';
  cwd?: string;
  // BOARD-014: Optional terminal dimensions (for testing)
  terminalWidth?: number;
  terminalHeight?: number;
}

// UNIFIED TABLE LAYOUT IMPLEMENTATION (ITF-004)
export const BoardView: React.FC<BoardViewProps> = ({ onExit, showStashPanel = true, showFilesPanel = true, focusedPanel: initialFocusedPanel = 'board', cwd, terminalWidth, terminalHeight }) => {
  const workUnits = useFspecStore(state => state.workUnits);
  const stashes = useFspecStore(state => state.stashes);
  const storeCwd = useFspecStore(state => state.cwd);
  const setCwd = useFspecStore(state => state.setCwd);
  const loadData = useFspecStore(state => state.loadData);
  const loadStashes = useFspecStore(state => state.loadStashes);
  const moveWorkUnitUp = useFspecStore(state => state.moveWorkUnitUp);
  const moveWorkUnitDown = useFspecStore(state => state.moveWorkUnitDown);

  const [focusedColumnIndex, setFocusedColumnIndex] = useState(0);
  const [selectedWorkUnitIndex, setSelectedWorkUnitIndex] = useState(0);
  const [viewMode, setViewMode] = useState<'board' | 'detail' | 'checkpoint-viewer' | 'changed-files-viewer'>('board');
  const [initialFocusSet, setInitialFocusSet] = useState(false);
  const [selectedWorkUnit, setSelectedWorkUnit] = useState<any>(null);
  const [focusedPanel, setFocusedPanel] = useState<'board' | 'stash' | 'files'>(initialFocusedPanel);

  const columns = [
    'backlog',
    'specifying',
    'testing',
    'implementing',
    'validating',
    'done',
    'blocked',
  ];

  // Set cwd if provided (for test isolation)
  useEffect(() => {
    if (cwd) {
      setCwd(cwd);
    }
  }, [cwd, setCwd]);

  // Enable mouse tracking for board view (TUI-010)
  useEffect(() => {
    process.stdout.write('\x1b[?1000h'); // Enable button event tracking
    return () => {
      process.stdout.write('\x1b[?1000l'); // Disable on unmount
    };
  }, []);

  // Load data on mount
  useEffect(() => {
    void loadData();
    void loadStashes();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Watch spec/work-units.json for changes and auto-refresh (BOARD-003)
  // NOTE: Watch the directory instead of the file to handle atomic rename operations
  // from the LockedFileManager (LOCK-002). Atomic renames create new inodes,
  // which breaks watchers on the original file.
  useEffect(() => {
    const specDir = path.join(storeCwd, 'spec');
    const workUnitsFileName = 'work-units.json';

    // Setup directory watcher (watches for rename events from atomic writes)
    const watcher = fs.watch(specDir, (eventType, filename) => {
      // Reload when work-units.json changes or is renamed (atomic write pattern)
      if (filename === workUnitsFileName) {
        void loadData();
      }
    });

    // Cleanup watcher on unmount
    return () => {
      watcher.close();
    };
  }, [storeCwd]);

  // Watch .git/refs/stash for stash changes using chokidar (BOARD-018: Cross-platform file watching)
  // NOTE: Use chokidar instead of fs.watch for reliable cross-platform atomic operation handling
  useEffect(() => {
    if (!showStashPanel) return;

    const stashPath = path.join(storeCwd, '.git', 'refs', 'stash');

    // Check if file exists before watching
    if (!fs.existsSync(stashPath)) return;

    // Chokidar watches specific file, handles atomic operations automatically
    const watcher = chokidar.watch(stashPath, {
      ignoreInitial: true,  // Don't trigger on initial scan
      persistent: false,
    });

    // Listen for all change events (chokidar normalizes across platforms)
    watcher.on('change', () => {
      void loadStashes();
    });

    // Add error handler to prevent silent failures (BOARD-018)
    watcher.on('error', (error) => {
      console.warn('Git refs watcher error:', error.message);
    });

    return () => {
      void watcher.close();
    };
  }, [showStashPanel, storeCwd]);

  // HEAD watcher removed (BOARD-018): Now handled by .git/ directory watcher above

  // Group work units by status
  const groupedWorkUnits = columns.map(status => {
    const units = workUnits.filter(wu => wu.status === status);
    const totalPoints = units.reduce((sum, wu) => {
      const estimate = typeof wu.estimate === 'number' ? wu.estimate : 0;
      return sum + estimate;
    }, 0);
    return { status, units, count: units.length, totalPoints };
  });

  // Auto-focus first non-empty column on initial load
  useEffect(() => {
    if (!initialFocusSet && workUnits.length > 0) {
      const firstNonEmptyIndex = groupedWorkUnits.findIndex(col => col.units.length > 0);
      if (firstNonEmptyIndex >= 0) {
        setFocusedColumnIndex(firstNonEmptyIndex);
        setInitialFocusSet(true);
      }
    }
  }, [workUnits, groupedWorkUnits, initialFocusSet]);

  // IPC server for checkpoint updates
  useEffect(() => {
    let server: Server | null = null;

    try {
      server = createIPCServer((message) => {
        if (message.type === 'checkpoint-changed') {
          void useFspecStore.getState().loadCheckpointCounts();
        }
      });

      const ipcPath = getIPCPath();
      server.listen(ipcPath);
    } catch (error) {
      // IPC server failed to start (non-fatal - TUI still works)
    }

    return () => {
      if (server) {
        cleanupIPCServer(server);
      }
    };
  }, []);

  // Compute currently selected work unit
  const currentlySelectedWorkUnit = (() => {
    const currentColumn = groupedWorkUnits[focusedColumnIndex];
    if (currentColumn && currentColumn.units.length > 0) {
      return currentColumn.units[selectedWorkUnitIndex] || null;
    }
    return null;
  })();

  // Handle keyboard navigation
  useInput((input, key) => {
    if (key.escape) {
      if (viewMode === 'detail' || viewMode === 'checkpoint-viewer' || viewMode === 'changed-files-viewer') {
        setViewMode('board');
        setSelectedWorkUnit(null);
        return;
      }
      onExit?.();
      return;
    }

    // C key to open checkpoint viewer (GIT-004)
    if (input === 'c' || input === 'C') {
      setViewMode('checkpoint-viewer');
      return;
    }

    // F key to open changed files viewer (GIT-004)
    if (input === 'f' || input === 'F') {
      setViewMode('changed-files-viewer');
      return;
    }

    // Tab key to switch panels (BOARD-003)
    if (key.tab) {
      if (focusedPanel === 'board') {
        setFocusedPanel('stash');
      } else if (focusedPanel === 'stash') {
        setFocusedPanel('files');
      } else {
        setFocusedPanel('board');
      }
      return;
    }
  });

  // Checkpoint viewer (GIT-004)
  if (viewMode === 'checkpoint-viewer') {
    return (
      <FullScreenWrapper>
        <CheckpointViewer
          onExit={() => setViewMode('board')}
        />
      </FullScreenWrapper>
    );
  }

  // Changed files viewer (GIT-004)
  if (viewMode === 'changed-files-viewer') {
    return (
      <FullScreenWrapper>
        <ChangedFilesViewer
          onExit={() => setViewMode('board')}
          terminalWidth={terminalWidth}
          terminalHeight={terminalHeight}
        />
      </FullScreenWrapper>
    );
  }

  // Work unit detail view
  if (viewMode === 'detail' && selectedWorkUnit) {
    const { stdout } = useStdout();
    const availableHeight = (terminalHeight || stdout?.rows || 24) - 10; // Reserve space for header and footer

    // Split description into lines for scrollable display
    const descriptionText = selectedWorkUnit.description || 'No description';
    const descriptionLines = descriptionText.split('\n');

    // Render a single line with selection indicator
    const renderLine = (line: string, _index: number, isSelected: boolean): React.ReactNode => {
      const indicator = isSelected ? '>' : ' ';
      return (
        <Box flexGrow={1}>
          <Text color={isSelected ? 'cyan' : 'white'}>
            {indicator} {line || ' '}
          </Text>
        </Box>
      );
    };

    return (
      <FullScreenWrapper>
        <Box flexDirection="column" padding={1} flexGrow={1} flexShrink={1}>
          <Text bold>{selectedWorkUnit.id} - {selectedWorkUnit.title}</Text>
          <Text>Type: {selectedWorkUnit.type}</Text>
          <Text>Status: {selectedWorkUnit.status}</Text>
          {selectedWorkUnit.estimate && <Text>Estimate: {selectedWorkUnit.estimate} points</Text>}
          <Text>{'\n'}Description:</Text>

          <VirtualList
            items={descriptionLines}
            height={availableHeight}
            renderItem={renderLine}
            showScrollbar={true}
            emptyMessage="No description"
          />

          <Text dimColor>{'\n'}Press ESC to return | Use ↑↓ to scroll | PgUp/PgDn, Home/End</Text>
        </Box>
      </FullScreenWrapper>
    );
  }

  return (
    <FullScreenWrapper>
      <UnifiedBoardLayout
        workUnits={workUnits}
        stashes={stashes}
        focusedColumnIndex={focusedColumnIndex}
        selectedWorkUnitIndex={selectedWorkUnitIndex}
        selectedWorkUnit={currentlySelectedWorkUnit}
        terminalWidth={terminalWidth}
        terminalHeight={terminalHeight}
        cwd={cwd}
        onColumnChange={(delta) => {
          setFocusedColumnIndex(prev => {
            const newIndex = prev + delta;
            if (newIndex < 0) return columns.length - 1;
            if (newIndex >= columns.length) return 0;
            return newIndex;
          });
          setSelectedWorkUnitIndex(0);
        }}
        onWorkUnitChange={(delta) => {
          const currentColumn = groupedWorkUnits[focusedColumnIndex];
          if (currentColumn.units.length > 0) {
            setSelectedWorkUnitIndex(prev => {
              const newIndex = prev + delta;
              if (newIndex < 0) return currentColumn.units.length - 1;
              if (newIndex >= currentColumn.units.length) return 0;
              return newIndex;
            });
          }
        }}
        onEnter={() => {
          // Handle Enter key based on focused panel (BOARD-003)
          if (focusedPanel === 'board') {
            const currentColumn = groupedWorkUnits[focusedColumnIndex];
            if (currentColumn.units.length > 0) {
              const workUnit = currentColumn.units[selectedWorkUnitIndex];
              setSelectedWorkUnit(workUnit);
              setViewMode('detail');
            }
          } else if (focusedPanel === 'stash' && stashes.length > 0) {
            setViewMode('stash-detail');
          } else if (focusedPanel === 'files' && (stagedFiles.length > 0 || unstagedFiles.length > 0)) {
            setViewMode('file-diff');
          }
        }}
        onMoveUp={async () => {
          // BOARD-010: Move work unit up with [ key
          const currentColumn = groupedWorkUnits[focusedColumnIndex];
          if (currentColumn.units.length > 0 && selectedWorkUnitIndex > 0) {
            const workUnit = currentColumn.units[selectedWorkUnitIndex];
            await moveWorkUnitUp(workUnit.id);
            await loadData();
            // BOARD-010: Move selection cursor up with the work unit
            setSelectedWorkUnitIndex(selectedWorkUnitIndex - 1);
          }
        }}
        onMoveDown={async () => {
          // BOARD-010: Move work unit down with ] key
          const currentColumn = groupedWorkUnits[focusedColumnIndex];
          if (currentColumn.units.length > 0 && selectedWorkUnitIndex < currentColumn.units.length - 1) {
            const workUnit = currentColumn.units[selectedWorkUnitIndex];
            await moveWorkUnitDown(workUnit.id);
            await loadData();
            // BOARD-010: Move selection cursor down with the work unit
            setSelectedWorkUnitIndex(selectedWorkUnitIndex + 1);
          }
        }}
      />
    </FullScreenWrapper>
  );
};
