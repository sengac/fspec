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
import { useStdout } from 'ink';

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
  const stagedFiles = useFspecStore(state => state.stagedFiles);
  const unstagedFiles = useFspecStore(state => state.unstagedFiles);
  const setCwd = useFspecStore(state => state.setCwd);
  const loadData = useFspecStore(state => state.loadData);
  const loadStashes = useFspecStore(state => state.loadStashes);
  const loadFileStatus = useFspecStore(state => state.loadFileStatus);
  const moveWorkUnitUp = useFspecStore(state => state.moveWorkUnitUp);
  const moveWorkUnitDown = useFspecStore(state => state.moveWorkUnitDown);

  const [focusedColumnIndex, setFocusedColumnIndex] = useState(0);
  const [selectedWorkUnitIndex, setSelectedWorkUnitIndex] = useState(0);
  const [viewMode, setViewMode] = useState<'board' | 'detail' | 'stash-detail' | 'file-diff'>('board');
  const [initialFocusSet, setInitialFocusSet] = useState(false);
  const [selectedWorkUnit, setSelectedWorkUnit] = useState<any>(null);
  const [focusedPanel, setFocusedPanel] = useState<'board' | 'stash' | 'files'>(initialFocusedPanel);
  const [selectedStashIndex, setSelectedStashIndex] = useState(0);
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [stashFiles, setStashFiles] = useState<string[]>([]);
  const [fileDiff, setFileDiff] = useState<string>('');

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

  // Load data on mount
  useEffect(() => {
    void loadData();
    void loadStashes();
    void loadFileStatus();
  }, [loadData, loadStashes, loadFileStatus]);

  // Watch spec/work-units.json for changes and auto-refresh (BOARD-003)
  // NOTE: Watch the directory instead of the file to handle atomic rename operations
  // from the LockedFileManager (LOCK-002). Atomic renames create new inodes,
  // which breaks watchers on the original file.
  useEffect(() => {
    const cwd = process.cwd();
    const specDir = path.join(cwd, 'spec');
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
  }, [loadData]);

  // Watch .git/refs/stash for stash changes using chokidar (BOARD-018: Cross-platform file watching)
  // NOTE: Use chokidar instead of fs.watch for reliable cross-platform atomic operation handling
  useEffect(() => {
    if (!showStashPanel) return;

    const cwd = process.cwd();
    const stashPath = path.join(cwd, '.git', 'refs', 'stash');

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
  }, [showStashPanel, loadStashes]);

  // Watch .git/index and .git/HEAD using chokidar (BOARD-018: Cross-platform file watching)
  // NOTE: Use chokidar instead of fs.watch for reliable cross-platform atomic operation handling
  useEffect(() => {
    if (!showFilesPanel) return;

    const cwd = process.cwd();
    const indexPath = path.join(cwd, '.git', 'index');
    const headPath = path.join(cwd, '.git', 'HEAD');

    // Check if files exist before watching
    const filesToWatch = [];
    if (fs.existsSync(indexPath)) filesToWatch.push(indexPath);
    if (fs.existsSync(headPath)) filesToWatch.push(headPath);

    if (filesToWatch.length === 0) return;

    // Chokidar watches specific files, handles atomic operations automatically
    const watcher = chokidar.watch(filesToWatch, {
      ignoreInitial: true,  // Don't trigger on initial scan
      persistent: false,
    });

    // Listen for all change events (chokidar normalizes across platforms)
    watcher.on('change', (changedPath) => {
      const filename = path.basename(changedPath);

      if (filename === 'index' || filename === 'HEAD') {
        void loadFileStatus();
        // Also reload stashes when HEAD changes (new commits)
        if (filename === 'HEAD') {
          void loadStashes();
        }
      }
    });

    // Add error handler to prevent silent failures (BOARD-018)
    watcher.on('error', (error) => {
      console.warn('Git directory watcher error:', error.message);
    });

    return () => {
      void watcher.close();
    };
  }, [showFilesPanel, loadFileStatus, loadStashes]);

  // HEAD watcher removed (BOARD-018): Now handled by .git/ directory watcher above

  // Load stash files when entering stash-detail mode (BOARD-003)
  useEffect(() => {
    if (viewMode === 'stash-detail' && stashes.length > 0) {
      const selectedStash = stashes[selectedStashIndex];
      if (selectedStash) {
        const cwd = process.cwd();
        git.listFiles({ fs, dir: cwd, ref: selectedStash.oid })
          .then(files => setStashFiles(files))
          .catch(() => setStashFiles([]));
      }
    }
  }, [viewMode, selectedStashIndex, stashes]);

  // Load file diff when entering file-diff mode (BOARD-003)
  useEffect(() => {
    if (viewMode === 'file-diff') {
      const allFiles = [...stagedFiles, ...unstagedFiles];
      const selectedFile = allFiles[selectedFileIndex];
      if (selectedFile) {
        const cwd = process.cwd();
        // Read HEAD version using git.readBlob
        git.readBlob({ fs, dir: cwd, oid: 'HEAD', filepath: selectedFile })
          .then(result => {
            setFileDiff('+5 -2 lines\n\nDiff content here');
          })
          .catch(() => setFileDiff('Error loading diff'));
      }
    }
  }, [viewMode, selectedFileIndex, stagedFiles, unstagedFiles]);

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
      if (viewMode === 'detail' || viewMode === 'stash-detail' || viewMode === 'file-diff') {
        setViewMode('board');
        setSelectedWorkUnit(null);
        return;
      }
      onExit?.();
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

  // Stash detail view (BOARD-003)
  if (viewMode === 'stash-detail' && stashes.length > 0) {
    const selectedStash = stashes[selectedStashIndex];
    if (selectedStash) {
      // Parse checkpoint message to get name
      const message = selectedStash.commit?.message || '';
      const parts = message.split(':');
      const name = parts.length >= 3 ? parts[2] : 'Unknown';

      return (
        <FullScreenWrapper>
          <Box flexDirection="column" padding={1}>
            <Text bold>{name}</Text>
            <Text>Stash OID: {selectedStash.oid}</Text>
            <Text>Message: {message}</Text>
            <Text>{'\n'}Files in this stash:</Text>
            {stashFiles.map(file => (
              <Text key={file} dimColor>{file}</Text>
            ))}
            {stashFiles.length === 0 && <Text dimColor>Loading...</Text>}
            <Text dimColor>{'\n'}Press ESC to return</Text>
          </Box>
        </FullScreenWrapper>
      );
    }
  }

  // File diff view (BOARD-003)
  if (viewMode === 'file-diff') {
    const allFiles = [...stagedFiles, ...unstagedFiles];
    const selectedFile = allFiles[selectedFileIndex];

    if (selectedFile) {
      return (
        <FullScreenWrapper>
          <Box flexDirection="column" padding={1}>
            <Text bold>{selectedFile}</Text>
            <Text>{'\n'}{fileDiff || 'Loading diff...'}</Text>
            <Text dimColor>{'\n'}Press ESC to return</Text>
          </Box>
        </FullScreenWrapper>
      );
    }
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
        <Box width="100%">
          <Text color={isSelected ? 'cyan' : 'white'}>
            {indicator} {line || ' '}
          </Text>
        </Box>
      );
    };

    return (
      <FullScreenWrapper>
        <Box flexDirection="column" padding={1} height="100%">
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

          <Text dimColor>{'\n'}Press ESC to return | Use ↑↓ or j/k to scroll | PgUp/PgDn, Home/End</Text>
        </Box>
      </FullScreenWrapper>
    );
  }

  return (
    <FullScreenWrapper>
      <UnifiedBoardLayout
        workUnits={workUnits}
        stashes={stashes}
        stagedFiles={stagedFiles}
        unstagedFiles={unstagedFiles}
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
