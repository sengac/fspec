/**
 * CheckpointViewer Component - three-pane viewer for checkpoints, files, and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 * - TUI-002: Checkpoint Viewer Three-Pane Layout
 * - INPUT-001: Uses centralized input handling with HIGH priority
 */

import React, { useState, useMemo, useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList.js';
import type { FileItem } from './FileDiffViewer.js';
import { Worker } from 'worker_threads';
import { parseDiff, DiffLine } from '../../git/diff-parser.js';
import { getWorkerPath } from '../../git/worker-path.js';
import { useFspecStore } from '../store/fspecStore.js';
import * as git from 'isomorphic-git';
import fs from 'fs';
import { join } from 'path';
import type { Checkpoint as GitCheckpoint } from '../../utils/git-checkpoint.js';
import {
  getCheckpointFilesChangedFromHead,
  deleteCheckpoint,
  deleteAllCheckpoints,
  restoreCheckpointFile,
  restoreCheckpoint,
} from '../../utils/git-checkpoint.js';
import {
  checkpointIndexDirExists,
  listCheckpointIndexFiles,
  getCheckpointIndexDir,
  readCheckpointIndexFile,
  isAutomaticCheckpoint,
  parseAutomaticCheckpointName,
} from '../../utils/checkpoint-index.js';
import { ConfirmationDialog } from '../../components/ConfirmationDialog.js';
import { StatusDialog } from '../../components/StatusDialog.js';
import { sendIPCMessage } from '../../utils/ipc.js';
import { useInputCompat, InputPriority } from '../input/index.js';

export interface Checkpoint {
  name: string;
  workUnitId: string;
  timestamp: string;
  stashRef: string;
  isAutomatic: boolean;
  files: string[];
  fileCount: number;
}

interface CheckpointViewerProps {
  onExit: () => void;
}

export const CheckpointViewer: React.FC<CheckpointViewerProps> = ({
  onExit,
}) => {
  const [focusedPane, setFocusedPane] = useState<
    'checkpoints' | 'files' | 'diff'
  >('checkpoints');
  const [selectedCheckpointIndex, setSelectedCheckpointIndex] = useState(0);
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);
  const [checkpoints, setCheckpoints] = useState<Checkpoint[]>([]);
  const [isLoadingCheckpoints, setIsLoadingCheckpoints] = useState(true);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [deleteMode, setDeleteMode] = useState<'single' | 'all'>('single');
  const [showRestoreDialog, setShowRestoreDialog] = useState(false);
  const [restoreMode, setRestoreMode] = useState<'single' | 'all'>('single');

  // StatusDialog state for restore progress
  const [showStatusDialog, setShowStatusDialog] = useState(false);
  const [statusDialogState, setStatusDialogState] = useState<'restoring' | 'complete' | 'error'>('restoring');
  const [currentFile, setCurrentFile] = useState<string>('');
  const [currentFileIndex, setCurrentFileIndex] = useState<number>(0);
  const [statusDialogError, setStatusDialogError] = useState<string | undefined>(undefined);

  const cwd = useFspecStore(state => state.cwd);

  // Checkpoint list and file list minimum width in characters (matches FileDiffViewer)
  const leftColumnMinWidth = 30;
  const fileListMinWidth = 30;

  // Worker thread reference
  const workerRef = useRef<Worker | null>(null);
  const pendingRequestId = useRef<string | null>(null);

  // Load all checkpoints from all work units
  useEffect(() => {
    const loadAllCheckpoints = async () => {
      setIsLoadingCheckpoints(true);
      try {
        // Use shared utility to list index files (handles ENOENT gracefully)
        const indexFiles = await listCheckpointIndexFiles(cwd);

        // No checkpoint files = no checkpoints
        if (indexFiles.length === 0) {
          setCheckpoints([]);
          setIsLoadingCheckpoints(false);
          return;
        }

        const allCheckpoints: Checkpoint[] = [];

        for (const indexFile of indexFiles) {
          const workUnitId = indexFile.replace('.json', '');

          // Use shared utility for reading index files (DRY)
          const index = await readCheckpointIndexFile(cwd, workUnitId);

          for (const cp of index.checkpoints) {
            const ref = `refs/fspec-checkpoints/${workUnitId}/${cp.name}`;

            try {
              // Resolve checkpoint ref to get OID
              const checkpointOid = await git.resolveRef({ fs, dir: cwd, ref });

              // Load files that differ from current HEAD
              const files = await getCheckpointFilesChangedFromHead(cwd, checkpointOid);

              // Parse checkpoint message to extract timestamp
              const match = cp.message.match(
                /^fspec-checkpoint:[^:]+:[^:]+:([^:]+)$/
              );
              const timestamp = match
                ? new Date(parseInt(match[1])).toISOString()
                : new Date().toISOString();

              allCheckpoints.push({
                name: cp.name,
                workUnitId,
                timestamp,
                stashRef: ref,
                isAutomatic: isAutomaticCheckpoint(cp.name),
                files,
                fileCount: files.length,
              });
            } catch (error) {
              // Skip checkpoints that can't be loaded
            }
          }
        }

        setCheckpoints(allCheckpoints);
      } catch (error) {
        setCheckpoints([]);
      } finally {
        setIsLoadingCheckpoints(false);
      }
    };

    void loadAllCheckpoints();
  }, [cwd]);

  // Sort checkpoints by timestamp (most recent first) and limit to 200
  const sortedCheckpoints = useMemo(() => {
    const sorted = [...checkpoints].sort((a, b) => {
      return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
    });
    // Limit to 200 most recent checkpoints for performance
    return sorted.slice(0, 200);
  }, [checkpoints]);

  // Get current checkpoint and files
  const currentCheckpoint = sortedCheckpoints[selectedCheckpointIndex];
  const files: FileItem[] = useMemo(() => {
    if (!currentCheckpoint) {
      return [];
    }
    return currentCheckpoint.files.map(f => ({
      path: f,
      status: 'checkpoint' as const,
    }));
  }, [currentCheckpoint, selectedCheckpointIndex]);

  // Initialize worker thread on mount
  useEffect(() => {
    const workerPath = getWorkerPath();

    try {
      workerRef.current = new Worker(workerPath);
    } catch (error) {
      // Worker initialization failed
    }

    return () => {
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  // Load git diff when selected file changes using worker thread
  useEffect(() => {
    const selectedFile = files[selectedFileIndex];

    if (!selectedFile) {
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    if (!workerRef.current) {
      setDiffContent('Worker thread not available');
      setIsLoadingDiff(false);
      return;
    }

    setIsLoadingDiff(true);

    const requestId = `${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;

    // Set up message handler for this request
    const messageHandler = (response: {
      id: string;
      diff?: string;
      error?: string;
    }) => {
      // Ignore responses from cancelled requests
      if (response.id !== pendingRequestId.current) {
        return;
      }

      if (response.error) {
        setDiffContent('Error loading diff');
      } else {
        const diffLength = response.diff?.length || 0;

        // Truncate large diffs to prevent UX hangs
        const MAX_DIFF_SIZE = 100000; // 100KB max
        let finalDiff = response.diff || 'No changes to display';
        if (diffLength > MAX_DIFF_SIZE) {
          const truncatedDiff = response.diff!.substring(0, MAX_DIFF_SIZE);
          const linesShown = truncatedDiff.split('\n').length;
          const totalLines = response.diff!.split('\n').length;
          finalDiff =
            truncatedDiff +
            `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${diffLength} chars)`;
        }

        setDiffContent(finalDiff);
      }

      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);

    // Send request to worker with checkpointRef to compare checkpoint vs HEAD
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
      checkpointRef: currentCheckpoint.stashRef, // Compare checkpoint file vs HEAD
    });

    // Cleanup function to cancel pending requests
    return () => {
      pendingRequestId.current = null;
      worker.off('message', messageHandler);
    };
  }, [selectedFileIndex, files, cwd, currentCheckpoint]);

  // Parse diff content into structured DiffLine objects
  const diffLines: DiffLine[] = useMemo(() => {
    if (isLoadingDiff) {
      return [
        { content: 'Loading diff...', type: 'context', changeGroup: null },
      ];
    }
    if (!diffContent) {
      return [];
    }
    return parseDiff(diffContent);
  }, [diffContent, isLoadingDiff]);

  // Handle delete confirmation
  const handleDeleteConfirm = async () => {
    if (deleteMode === 'single') {
      const checkpoint = sortedCheckpoints[selectedCheckpointIndex];
      if (checkpoint) {
        await deleteCheckpoint({
          workUnitId: checkpoint.workUnitId,
          checkpointName: checkpoint.name,
          cwd,
        });

        // Send IPC notification
        await sendIPCMessage({ type: 'checkpoint-changed' });

        // Reload checkpoints
        const updatedCheckpoints = checkpoints.filter(
          cp => cp.name !== checkpoint.name
        );
        setCheckpoints(updatedCheckpoints);

        // Select next checkpoint or exit if none remain
        if (updatedCheckpoints.length === 0) {
          onExit();
        } else {
          setSelectedCheckpointIndex(
            Math.min(selectedCheckpointIndex, updatedCheckpoints.length - 1)
          );
        }
      }
    } else {
      // Delete ALL checkpoints across all work units
      // Get unique work unit IDs from all checkpoints
      const uniqueWorkUnitIds = [
        ...new Set(checkpoints.map(cp => cp.workUnitId)),
      ];

      // Delete checkpoints for each work unit
      for (const workUnitId of uniqueWorkUnitIds) {
        await deleteAllCheckpoints({
          workUnitId,
          cwd,
        });
      }

      // Send IPC notification
      await sendIPCMessage({ type: 'checkpoint-changed' });

      // Exit to board after deleting all
      onExit();
    }

    setShowDeleteDialog(false);
  };

  const handleDeleteCancel = () => {
    setShowDeleteDialog(false);
  };

  // Helper function to reload diff after restore
  const reloadDiffAfterRestore = (checkpoint: GitCheckpoint) => {
    const selectedFile = files[selectedFileIndex];
    if (!selectedFile || !workerRef.current) return;

    setIsLoadingDiff(true);
    const requestId = `${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;
    const messageHandler = (response: {
      id: string;
      diff?: string;
      error?: string;
    }) => {
      if (response.id !== pendingRequestId.current) return;
      if (response.error) {
        setDiffContent('Error loading diff');
      } else {
        setDiffContent(response.diff || 'No changes to display');
      }
      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
      checkpointRef: checkpoint.stashRef,
    });
  };

  // Handle restore confirmation
  const handleRestoreConfirm = async () => {
    if (restoreMode === 'single') {
      const checkpoint = sortedCheckpoints[selectedCheckpointIndex];
      const file = files[selectedFileIndex];
      if (checkpoint && file) {
        // Show StatusDialog for single file restore
        setShowStatusDialog(true);
        setStatusDialogState('restoring');
        setCurrentFile(file.path);
        setCurrentFileIndex(1);
        setStatusDialogError(undefined);
        setShowRestoreDialog(false);

        // Resolve ref to OID
        const checkpointOid = await git.resolveRef({
          fs,
          dir: cwd,
          ref: checkpoint.stashRef,
        });

        const result = await restoreCheckpointFile({
          cwd,
          checkpointOid,
          filepath: file.path,
          force: true,
        });

        if (result.success) {
          // Send IPC notification
          await sendIPCMessage({ type: 'checkpoint-changed' });

          // Reload diff to show updated comparison
          reloadDiffAfterRestore(checkpoint);

          // Show completion state
          setStatusDialogState('complete');
        } else {
          // Show error state
          setStatusDialogState('error');
          setStatusDialogError(result.systemReminder || 'Failed to restore file');
        }
      }
    } else {
      // Restore all files with progress tracking
      const checkpoint = sortedCheckpoints[selectedCheckpointIndex];
      if (checkpoint) {
        // Show StatusDialog
        setShowStatusDialog(true);
        setStatusDialogState('restoring');
        setCurrentFileIndex(0);
        setStatusDialogError(undefined);
        setShowRestoreDialog(false);

        // Resolve checkpoint OID
        const checkpointOid = await git.resolveRef({
          fs,
          dir: cwd,
          ref: checkpoint.stashRef,
        });

        // Restore files one by one
        const filesToRestore = checkpoint.files;
        let hasError = false;

        for (let i = 0; i < filesToRestore.length; i++) {
          const filepath = filesToRestore[i];
          setCurrentFile(filepath);
          setCurrentFileIndex(i + 1);

          const result = await restoreCheckpointFile({
            cwd,
            checkpointOid,
            filepath,
            force: true,
          });

          if (!result.success) {
            // Show error state
            setStatusDialogState('error');
            setStatusDialogError(`Failed to restore ${filepath}: ${result.systemReminder || 'Unknown error'}`);
            hasError = true;
            break;
          }
        }

        if (!hasError) {
          // Send IPC notification
          await sendIPCMessage({ type: 'checkpoint-changed' });

          // Reload diff
          reloadDiffAfterRestore(checkpoint);

          // Show completion state
          setStatusDialogState('complete');
        }
      }
    }
  };

  const handleRestoreCancel = () => {
    setShowRestoreDialog(false);
  };

  const handleStatusDialogClose = () => {
    setShowStatusDialog(false);
  };

  // Handle keyboard input with HIGH priority (full-screen viewer)
  useInputCompat({
    id: 'checkpoint-viewer',
    priority: InputPriority.HIGH,
    description: 'Checkpoint viewer keyboard navigation',
    isActive: !showDeleteDialog && !showRestoreDialog,
    handler: (input, key) => {
      if (key.escape) {
        onExit();
        return true;
      }

      // R key for single file restore (files pane only)
      if (input === 'r' || input === 'R') {
        if (focusedPane === 'files' && files.length > 0) {
          setRestoreMode('single');
          setShowRestoreDialog(true);
        }
        return true;
      }

      // T key for restore all files
      if (input === 't' || input === 'T') {
        if (
          (focusedPane === 'checkpoints' || focusedPane === 'files') &&
          sortedCheckpoints.length > 0
        ) {
          setRestoreMode('all');
          setShowRestoreDialog(true);
        }
        return true;
      }

      // D key for single deletion
      if (input === 'd' || input === 'D') {
        if (sortedCheckpoints.length > 0) {
          setDeleteMode('single');
          setShowDeleteDialog(true);
        }
        return true;
      }

      // A key for delete ALL
      if (input === 'a' || input === 'A') {
        if (sortedCheckpoints.length > 0) {
          setDeleteMode('all');
          setShowDeleteDialog(true);
        }
        return true;
      }

      // Tab key or right arrow to cycle forward through three panes
      if (key.tab || key.rightArrow) {
        setFocusedPane(prev => {
          if (prev === 'checkpoints') return 'files';
          if (prev === 'files') return 'diff';
          return 'checkpoints';
        });
        return true;
      }

      // Left arrow to cycle backward through three panes
      if (key.leftArrow) {
        setFocusedPane(prev => {
          if (prev === 'checkpoints') return 'diff';
          if (prev === 'files') return 'checkpoints';
          return 'files';
        });
        return true;
      }

      // Up/down arrow key navigation handled by VirtualList when focused
      return false;
    },
  });

  // Loading state
  if (isLoadingCheckpoints) {
    return (
      <Box
        flexDirection="column"
        flexGrow={1}
        borderStyle="single"
        borderTop={false}
        borderLeft={false}
        borderRight={false}
        borderBottom={true}
      >
        <Box flexDirection="column" flexGrow={1}>
          <Box
            flexDirection="row"
            flexGrow={1}
            flexBasis={0}
            borderStyle="single"
            borderTop={false}
            borderBottom={true}
            borderLeft={false}
            borderRight={false}
          >
            <Box
              flexDirection="column"
              flexGrow={1}
              flexBasis={0}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={true}
              borderBottom={false}
            >
              <Box
                backgroundColor={
                  focusedPane === 'checkpoints' ? 'green' : undefined
                }
                borderStyle="single"
                borderTop={false}
                borderLeft={false}
                borderRight={false}
                borderBottom={true}
              >
                <Text
                  bold={focusedPane !== 'checkpoints'}
                  color={focusedPane === 'checkpoints' ? 'black' : 'white'}
                >
                  Checkpoints
                </Text>
              </Box>
              <Text wrap="truncate">Loading checkpoints...</Text>
            </Box>
            <Box flexDirection="column" flexGrow={2} flexBasis={0}>
              <Box
                backgroundColor={focusedPane === 'files' ? 'green' : undefined}
                borderStyle="single"
                borderTop={false}
                borderLeft={false}
                borderRight={false}
                borderBottom={true}
              >
                <Text
                  bold={focusedPane !== 'files'}
                  color={focusedPane === 'files' ? 'black' : 'white'}
                >
                  Files
                </Text>
              </Box>
              <Text wrap="truncate">-</Text>
            </Box>
          </Box>
          <Box flexDirection="column" flexGrow={2} flexBasis={0}>
            <Box
              backgroundColor={focusedPane === 'diff' ? 'green' : undefined}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={false}
              borderBottom={true}
            >
              <Text
                bold={focusedPane !== 'diff'}
                color={focusedPane === 'diff' ? 'black' : 'white'}
              >
                Diff
              </Text>
            </Box>
            <Text wrap="truncate">-</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Empty state
  if (sortedCheckpoints.length === 0) {
    return (
      <Box
        flexDirection="column"
        flexGrow={1}
        borderStyle="single"
        borderTop={false}
        borderLeft={false}
        borderRight={false}
        borderBottom={true}
      >
        <Box flexDirection="column" flexGrow={1}>
          {/* Top row: Checkpoint list (left) + File list (right) - 33% height */}
          <Box
            flexDirection="row"
            flexGrow={1}
            flexBasis={0}
            borderStyle="single"
            borderTop={false}
            borderBottom={true}
            borderLeft={false}
            borderRight={false}
          >
            {/* Checkpoint list pane (left) - 33% of horizontal space via flexGrow ratio */}
            <Box
              flexDirection="column"
              flexGrow={1}
              flexBasis={0}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={true}
              borderBottom={false}
            >
              <Box
                backgroundColor={
                  focusedPane === 'checkpoints' ? 'green' : undefined
                }
                borderStyle="single"
                borderTop={false}
                borderLeft={false}
                borderRight={false}
                borderBottom={true}
              >
                <Text
                  bold={focusedPane !== 'checkpoints'}
                  color={focusedPane === 'checkpoints' ? 'black' : 'white'}
                >
                  Checkpoints
                </Text>
              </Box>
              <Text wrap="truncate">No checkpoints available</Text>
            </Box>
            {/* File list pane (right) - 67% of horizontal space via flexGrow ratio */}
            <Box flexDirection="column" flexGrow={2} flexBasis={0}>
              <Box
                backgroundColor={focusedPane === 'files' ? 'green' : undefined}
                borderStyle="single"
                borderTop={false}
                borderLeft={false}
                borderRight={false}
                borderBottom={true}
              >
                <Text
                  bold={focusedPane !== 'files'}
                  color={focusedPane === 'files' ? 'black' : 'white'}
                >
                  Files
                </Text>
              </Box>
              <Text wrap="truncate">No files</Text>
            </Box>
          </Box>

          {/* Bottom: Diff pane - 67% height */}
          <Box flexDirection="column" flexGrow={2} flexBasis={0}>
            <Box
              backgroundColor={focusedPane === 'diff' ? 'green' : undefined}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={false}
              borderBottom={true}
            >
              <Text
                bold={focusedPane !== 'diff'}
                color={focusedPane === 'diff' ? 'black' : 'white'}
              >
                Diff
              </Text>
            </Box>
            <Text wrap="truncate">Select a checkpoint to view files</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render checkpoint item with compact name
  const renderCheckpointItem = (
    checkpoint: Checkpoint,
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    // Extract compact name from checkpoint name (e.g., "TUI-001-auto-testing" -> "TUI-001: Testing")
    const parsed = parseAutomaticCheckpointName(checkpoint.name);
    let displayName: string;

    if (parsed) {
      // Automatic checkpoint - format as "WORK-001: Testing"
      const phase = parsed.state.charAt(0).toUpperCase() + parsed.state.slice(1).toLowerCase();
      displayName = `${parsed.workUnitId}: ${phase}`;
    } else {
      // Manual checkpoint - use name directly
      displayName = checkpoint.name;
    }

    return (
      <Box flexGrow={1}>
        <Text color={isSelected ? 'cyan' : 'white'} wrap="truncate">
          {displayName}
        </Text>
      </Box>
    );
  };

  // Render diff line with syntax highlighting
  const renderDiffLine = (
    line: DiffLine,
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    let textColor: 'white' | 'cyan' = 'white';
    let backgroundColor: string | undefined;

    // Determine colors based on line type
    if (line.type === 'hunk') {
      textColor = 'cyan';
    } else if (line.type === 'removed') {
      textColor = 'white';
      backgroundColor = '#8B0000'; // Dark red
    } else if (line.type === 'added') {
      textColor = 'white';
      backgroundColor = '#006400'; // Dark green
    }

    // Apply selection styling if focused
    const selectionColor =
      isSelected && focusedPane === 'diff' ? 'cyan' : textColor;
    const selectionInverse = isSelected && focusedPane === 'diff';

    return (
      <Box flexGrow={1}>
        <Text
          color={selectionInverse ? selectionColor : textColor}
          backgroundColor={backgroundColor}
          inverse={selectionInverse}
          wrap="truncate"
        >
          {line.content}
        </Text>
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Three-pane layout: Top row (checkpoint list + file list) + Bottom diff pane */}
      <Box
        flexDirection="column"
        flexGrow={1}
        borderStyle="single"
        borderTop={false}
        borderLeft={false}
        borderRight={false}
        borderBottom={true}
      >
        <Box flexDirection="column" flexGrow={1}>
          {/* Top row: Checkpoint list (left) + File list (right) side-by-side - 33% height via flexGrow ratio */}
          <Box
            flexDirection="row"
            flexGrow={1}
            flexBasis={0}
            borderStyle="single"
            borderTop={false}
            borderBottom={true}
            borderLeft={false}
            borderRight={false}
          >
            {/* Checkpoint list pane (left) - 33% of horizontal space via flexGrow ratio */}
            <Box
              flexDirection="column"
              flexGrow={1}
              flexBasis={0}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={true}
              borderBottom={false}
            >
              {/* Checkpoint list heading */}
              <Box
                backgroundColor={
                  focusedPane === 'checkpoints' ? 'green' : undefined
                }
                borderStyle="single"
                borderTop={false}
                borderLeft={false}
                borderRight={false}
                borderBottom={true}
              >
                <Text
                  bold={focusedPane !== 'checkpoints'}
                  color={focusedPane === 'checkpoints' ? 'black' : 'white'}
                >
                  Checkpoints
                </Text>
              </Box>
              <VirtualList
                items={sortedCheckpoints}
                renderItem={renderCheckpointItem}
                showScrollbar={focusedPane === 'checkpoints'}
                isFocused={focusedPane === 'checkpoints' && !showDeleteDialog}
                heightAdjustment={-1}
                onFocus={(checkpoint, index) => {
                  setSelectedCheckpointIndex(index);
                  setSelectedFileIndex(0); // Reset file selection when checkpoint changes
                }}
              />
            </Box>

            {/* File list pane (right) - 67% of horizontal space via flexGrow ratio */}
            <Box flexDirection="column" flexGrow={2} flexBasis={0}>
              {/* File list heading */}
              <Box
                backgroundColor={focusedPane === 'files' ? 'green' : undefined}
                borderStyle="single"
                borderTop={false}
                borderLeft={false}
                borderRight={false}
                borderBottom={true}
              >
                <Text
                  bold={focusedPane !== 'files'}
                  color={focusedPane === 'files' ? 'black' : 'white'}
                  wrap="truncate"
                >
                  Files{sortedCheckpoints[selectedCheckpointIndex]?.name ? `: ${sortedCheckpoints[selectedCheckpointIndex].name}` : ''}
                </Text>
              </Box>
              <VirtualList
                items={files}
                renderItem={(file, index, isSelected) => {
                  const indicator = isSelected ? '>' : ' ';
                  return (
                    <Box flexGrow={1}>
                      <Text
                        color={isSelected ? 'cyan' : 'white'}
                        wrap="truncate"
                      >
                        {indicator} {file.path}
                      </Text>
                    </Box>
                  );
                }}
                showScrollbar={focusedPane === 'files'}
                isFocused={focusedPane === 'files' && !showDeleteDialog}
                heightAdjustment={-1}
                onFocus={(file, index) => {
                  setSelectedFileIndex(index);
                }}
              />
            </Box>
          </Box>

          {/* Bottom: Diff pane only - 67% height via flexGrow ratio */}
          <Box flexDirection="column" flexGrow={2} flexBasis={0}>
            {/* Diff pane heading */}
            <Box
              backgroundColor={focusedPane === 'diff' ? 'green' : undefined}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={false}
              borderBottom={true}
            >
              <Text
                bold={focusedPane !== 'diff'}
                color={focusedPane === 'diff' ? 'black' : 'white'}
                wrap="truncate"
              >
                Diff{files[selectedFileIndex]?.path ? `: ${files[selectedFileIndex].path}` : ''}
              </Text>
            </Box>
            <VirtualList
              items={diffLines}
              renderItem={renderDiffLine}
              showScrollbar={focusedPane === 'diff'}
              isFocused={focusedPane === 'diff' && !showDeleteDialog}
              selectionMode="scroll"
            />
          </Box>
        </Box>
      </Box>

      {/* Footer */}
      <Box>
        <Text dimColor>
          D: Delete Checkpoint | A: Delete All Checkpoints | T: Restore All Files | R: Restore Single File
        </Text>
      </Box>

      {/* Confirmation Dialog */}
      {showDeleteDialog && sortedCheckpoints.length > 0 && (
        <ConfirmationDialog
          message={
            deleteMode === 'single'
              ? `Delete checkpoint '${sortedCheckpoints[selectedCheckpointIndex]?.name}'?`
              : `Delete ALL ${sortedCheckpoints.length} checkpoints across all work units?`
          }
          description={
            deleteMode === 'all'
              ? 'This will permanently delete all checkpoints. This cannot be undone.'
              : undefined
          }
          confirmMode={deleteMode === 'single' ? 'yesno' : 'typed'}
          typedPhrase={deleteMode === 'all' ? 'DELETE ALL' : undefined}
          riskLevel={deleteMode === 'single' ? 'medium' : 'high'}
          onConfirm={handleDeleteConfirm}
          onCancel={handleDeleteCancel}
          isActive={true}
        />
      )}

      {showRestoreDialog && sortedCheckpoints.length > 0 && (
        <ConfirmationDialog
          message={
            restoreMode === 'single'
              ? `Restore ${files[selectedFileIndex]?.path} from checkpoint '${sortedCheckpoints[selectedCheckpointIndex]?.name}'?`
              : `Restore ALL ${files.length} files from checkpoint '${sortedCheckpoints[selectedCheckpointIndex]?.name}'?`
          }
          description="Overwrite warning: Current file content will be replaced. Uncommitted changes will be LOST."
          confirmMode="yesno"
          riskLevel={restoreMode === 'single' ? 'medium' : 'high'}
          onConfirm={handleRestoreConfirm}
          onCancel={handleRestoreCancel}
          isActive={true}
        />
      )}

      {showStatusDialog && (
        <StatusDialog
          currentItem={currentFile}
          currentIndex={currentFileIndex}
          totalItems={restoreMode === 'single' ? 1 : (sortedCheckpoints[selectedCheckpointIndex]?.files.length || 0)}
          status={statusDialogState}
          errorMessage={statusDialogError}
          onClose={handleStatusDialogClose}
        />
      )}
    </Box>
  );
};
