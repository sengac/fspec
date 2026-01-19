/**
 * BoardView - Interactive Kanban Board Component
 *
 * Coverage:
 * - BOARD-002: Interactive Kanban board CLI
 * - BOARD-003: Real-time board updates with git stash and file inspection
 * - ITF-004: Fix TUI Kanban column layout to match table style
 * - REFAC-004: Integrate attachment server with TUI (BoardView lifecycle)
 */

import React, { useEffect, useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { useFspecStore } from '../store/fspecStore';
import fs from 'fs';
import path from 'path';
import chokidar from 'chokidar';
import { UnifiedBoardLayout } from './UnifiedBoardLayout';
import { FullScreenWrapper } from './FullScreenWrapper';
import { CheckpointViewer } from './CheckpointViewer';
import { ChangedFilesViewer } from './ChangedFilesViewer';
import { AttachmentDialog } from './AttachmentDialog';
import { AgentView } from './AgentView';
import { createIPCServer, cleanupIPCServer, getIPCPath } from '../../utils/ipc';
import type { Server } from 'net';
import type { Server as HttpServer } from 'http';
import { logger } from '../../utils/logger';
import { openInBrowser } from '../../utils/openBrowser';
import { startAttachmentServer, stopAttachmentServer, getServerPort } from '../../server/attachment-server';

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
  const storeCwd = useFspecStore(state => state.cwd);
  const setCwd = useFspecStore(state => state.setCwd);
  const loadData = useFspecStore(state => state.loadData);
  const loadCheckpointCounts = useFspecStore(state => state.loadCheckpointCounts);
  const moveWorkUnitUp = useFspecStore(state => state.moveWorkUnitUp);
  const moveWorkUnitDown = useFspecStore(state => state.moveWorkUnitDown);
  const error = useFspecStore(state => state.error);
  const isLoaded = useFspecStore(state => state.isLoaded);

  const [focusedColumnIndex, setFocusedColumnIndex] = useState(0);
  const [selectedWorkUnitIndex, setSelectedWorkUnitIndex] = useState(0);
  const [viewMode, setViewMode] = useState<'board' | 'checkpoint-viewer' | 'changed-files-viewer' | 'agent'>('board');
  const [initialFocusSet, setInitialFocusSet] = useState(false);
  const [selectedWorkUnit, setSelectedWorkUnit] = useState<any>(null);
  const [focusedPanel, setFocusedPanel] = useState<'board' | 'stash' | 'files'>(initialFocusedPanel);
  const [showAttachmentDialog, setShowAttachmentDialog] = useState(false);
  const [attachmentServerPort, setAttachmentServerPort] = useState<number | null>(null);

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
  // Re-enable when returning from AgentView/CheckpointViewer/ChangedFilesViewer
  // since those views disable mouse tracking in their cleanup
  useEffect(() => {
    if (viewMode === 'board') {
      process.stdout.write('\x1b[?1000h'); // Enable button event tracking
      return () => {
        process.stdout.write('\x1b[?1000l'); // Disable on unmount
      };
    }
  }, [viewMode]);

  // Load data on mount
  useEffect(() => {
    void loadData();
    void loadCheckpointCounts();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Watch spec/work-units.json for changes and auto-refresh (BOARD-003, REFAC-004)
  // NOTE: Use chokidar instead of fs.watch for reliable cross-platform atomic operation handling
  // Atomic renames from LockedFileManager (LOCK-002) are handled automatically by chokidar
  useEffect(() => {
    const workUnitsPath = path.join(storeCwd, 'spec', 'work-units.json');

    // Check if file exists before watching
    if (!fs.existsSync(workUnitsPath)) return;

    // Chokidar watches specific file, handles atomic operations automatically
    const watcher = chokidar.watch(workUnitsPath, {
      ignoreInitial: true,  // Don't trigger on initial scan
      persistent: false,
    });

    // Listen for all change events (chokidar normalizes across platforms)
    watcher.on('change', () => {
      void loadData();
    });

    // Add error handler to prevent silent failures
    watcher.on('error', (error) => {
      console.warn('Work units watcher error:', error.message);
    });

    // Cleanup watcher on unmount
    return () => {
      void watcher.close();
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
      void loadCheckpointCounts();
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
      logger.error(`[BOARDVIEW] Failed to start IPC server: ${error}`);
    }

    return () => {
      if (server) {
        cleanupIPCServer(server);
      }
    };
  }, []);

  // Attachment server for rendering markdown/mermaid attachments (REFAC-004)
  useEffect(() => {
    let httpServer: HttpServer | null = null;

    const initServer = async () => {
      try {
        httpServer = await startAttachmentServer({
          cwd: storeCwd,
          port: 0, // Random available port
        });

        const port = getServerPort(httpServer);
        if (port) {
          setAttachmentServerPort(port);
        }
      } catch (error) {
        // Server startup failure is non-fatal - TUI continues working (REFAC-004 business rule 6)
        logger.warn(`[BoardView] Failed to start attachment server: ${error}`);
      }
    };

    void initServer();

    return () => {
      if (httpServer) {
        void stopAttachmentServer(httpServer).catch((error) => {
          logger.error(`[BoardView] Error stopping attachment server: ${error}`);
        });
      }
    };
  }, [storeCwd]);

  // Compute currently selected work unit
  const currentlySelectedWorkUnit = (() => {
    const currentColumn = groupedWorkUnits[focusedColumnIndex];
    if (currentColumn && currentColumn.units.length > 0) {
      return currentColumn.units[selectedWorkUnitIndex] || null;
    }
    return null;
  })();

  // Helper: Check if selected work unit has attachments
  const hasAttachments = () => {
    return !!(currentlySelectedWorkUnit &&
              currentlySelectedWorkUnit.attachments &&
              currentlySelectedWorkUnit.attachments.length > 0);
  };

  // Handle keyboard navigation
  useInput((input, key) => {
    if (key.escape) {
      if (viewMode === 'checkpoint-viewer' || viewMode === 'changed-files-viewer') {
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

    // D key to open FOUNDATION.md in browser (TUI-029)
    if (input === 'd' || input === 'D') {
      const foundationPath = 'spec/FOUNDATION.md';

      if (attachmentServerPort) {
        const url = `http://localhost:${attachmentServerPort}/view/${foundationPath}`;
        openInBrowser({ url, wait: false }).catch((error: Error) => {
          logger.error(`[BoardView] Failed to open FOUNDATION.md: ${error.message}`);
        });
      }
      return;
    }

    // A key to open attachment dialog (TUI-019)
    if (input === 'a' || input === 'A') {
      if (hasAttachments()) {
        setShowAttachmentDialog(true);
      }
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
  }, { isActive: viewMode === 'board' && !showAttachmentDialog });

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

  // Agent view (NAPI-002)
  if (viewMode === 'agent') {
    return (
      <FullScreenWrapper>
        <AgentView
          onExit={() => setViewMode('board')}
          workUnitId={selectedWorkUnit?.id}
        />
      </FullScreenWrapper>
    );
  }

  // Display loading state while data is being loaded (BUG-072)
  if (!isLoaded && !error) {
    // Loading view component with ESC key handler
    const LoadingView = () => {
      useInput((input, key) => {
        if (key.escape) {
          onExit?.();
        }
      }, { isActive: true });

      return (
        <Box flexDirection="column" padding={1}>
          <Text>Loading fspec board...</Text>
          <Text dimColor>{'\n'}Press ESC to exit</Text>
        </Box>
      );
    };

    return (
      <FullScreenWrapper>
        <LoadingView />
      </FullScreenWrapper>
    );
  }

  // Display error state if data loading failed (BUG-072)
  if (error) {
    // Error view component with ESC key handler
    const ErrorView = () => {
      useInput((input, key) => {
        if (key.escape) {
          onExit?.();
        }
      }, { isActive: true });

      return (
        <Box flexDirection="column" padding={1}>
          <Text color="red">Error loading board:</Text>
          <Text>{error}</Text>
          <Text dimColor>{'\n'}Press ESC to exit</Text>
        </Box>
      );
    };

    return (
      <FullScreenWrapper>
        <ErrorView />
      </FullScreenWrapper>
    );
  }

  return (
    <FullScreenWrapper>
      <UnifiedBoardLayout
        workUnits={workUnits}
        focusedColumnIndex={focusedColumnIndex}
        selectedWorkUnitIndex={selectedWorkUnitIndex}
        selectedWorkUnit={currentlySelectedWorkUnit}
        terminalWidth={terminalWidth}
        terminalHeight={terminalHeight}
        cwd={cwd}
        isDialogOpen={showAttachmentDialog}
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
          // Handle Enter key - open agent view for selected work unit
          if (focusedPanel === 'board') {
            const currentColumn = groupedWorkUnits[focusedColumnIndex];
            if (currentColumn.units.length > 0) {
              const workUnit = currentColumn.units[selectedWorkUnitIndex];
              setSelectedWorkUnit(workUnit);
              setViewMode('agent');
            }
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

      {/* TUI-019: Attachment selection dialog */}
      {showAttachmentDialog && hasAttachments() && (
        <AttachmentDialog
          attachments={currentlySelectedWorkUnit!.attachments!}
          onSelect={(attachment) => {
            // REFAC-004: Use HTTP URL if attachment server is running, otherwise fall back to file://
            let url: string;
            if (attachmentServerPort) {
              url = `http://localhost:${attachmentServerPort}/view/${attachment}`;
            } else {
              const absolutePath = path.isAbsolute(attachment)
                ? attachment
                : path.resolve(cwd || process.cwd(), attachment);
              url = `file://${absolutePath}`;
            }

            openInBrowser({ url, wait: false }).catch((error: Error) => {
              logger.error(`[BoardView] Failed to open attachment: ${error.message}`);
            });
            setShowAttachmentDialog(false);
          }}
          onClose={() => setShowAttachmentDialog(false)}
        />
      )}
    </FullScreenWrapper>
  );
};
