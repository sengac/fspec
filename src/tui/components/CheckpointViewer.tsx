/**
 * CheckpointViewer Component - three-pane viewer for checkpoints, files, and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 * - TUI-002: Checkpoint Viewer Three-Pane Layout
 */

import React, { useState, useMemo, useEffect, useRef } from 'react';
import { Box, Text, useInput } from 'ink';
import { VirtualList } from './VirtualList';
import type { FileItem } from './FileDiffViewer';
import { logger } from '../../utils/logger';
import { Worker } from 'worker_threads';
import { join } from 'path';
import { parseDiff, DiffLine } from '../../git/diff-parser';
import { useFspecStore } from '../store/fspecStore';
import * as git from 'isomorphic-git';
import fs from 'fs';
import type { Checkpoint as GitCheckpoint } from '../../utils/git-checkpoint';
import { getCheckpointChangedFiles } from '../../utils/git-checkpoint';

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
  const [focusedPane, setFocusedPane] = useState<'checkpoints' | 'files' | 'diff'>('checkpoints');
  const [selectedCheckpointIndex, setSelectedCheckpointIndex] = useState(0);
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);
  const [checkpoints, setCheckpoints] = useState<Checkpoint[]>([]);
  const [isLoadingCheckpoints, setIsLoadingCheckpoints] = useState(true);

  const cwd = useFspecStore(state => state.cwd);

  // Checkpoint list and file list minimum width in characters (matches FileDiffViewer)
  const leftColumnMinWidth = 30;
  const fileListMinWidth = 30;

  // Worker thread reference
  const workerRef = useRef<Worker | null>(null);
  const pendingRequestId = useRef<string | null>(null);
  const componentId = useRef(`CheckpointViewer-${Date.now()}`);

  // Load all checkpoints from all work units
  useEffect(() => {
    const loadAllCheckpoints = async () => {
      logger.info(`[${componentId.current}] Starting checkpoint loading from cwd: ${cwd}`);
      setIsLoadingCheckpoints(true);
      try {
        const indexDir = join(cwd, '.git', 'fspec-checkpoints-index');
        logger.info(`[${componentId.current}] Checking index directory: ${indexDir}`);

        // Check if index directory exists
        if (!fs.existsSync(indexDir)) {
          logger.warn(`[${componentId.current}] Index directory does not exist, no checkpoints to load`);
          setCheckpoints([]);
          setIsLoadingCheckpoints(false);
          return;
        }

        // Read all work unit index files
        const indexFiles = fs.readdirSync(indexDir).filter(f => f.endsWith('.json'));
        logger.info(`[${componentId.current}] Found ${indexFiles.length} index files: ${indexFiles.join(', ')}`);
        const allCheckpoints: Checkpoint[] = [];

        for (const indexFile of indexFiles) {
          const workUnitId = indexFile.replace('.json', '');
          const indexPath = join(indexDir, indexFile);
          logger.info(`[${componentId.current}] Processing index file: ${indexFile} for work unit: ${workUnitId}`);

          const content = fs.readFileSync(indexPath, 'utf-8');
          const index = JSON.parse(content) as { checkpoints: { name: string; message: string }[] };
          logger.info(`[${componentId.current}] Found ${index.checkpoints.length} checkpoints in ${indexFile}`);

          for (const cp of index.checkpoints) {
            const ref = `refs/fspec-checkpoints/${workUnitId}/${cp.name}`;
            logger.info(`[${componentId.current}] Loading checkpoint: ${cp.name} with ref: ${ref}`);

            try {
              // Resolve checkpoint ref to get OID
              const checkpointOid = await git.resolveRef({ fs, dir: cwd, ref });
              logger.info(`[${componentId.current}] Resolved checkpoint ${cp.name} to OID: ${checkpointOid}`);

              // Load changed files from checkpoint (not all files)
              logger.info(`[${componentId.current}] Getting changed files for checkpoint ${cp.name}`);
              const files = await getCheckpointChangedFiles(cwd, checkpointOid);
              logger.info(`[${componentId.current}] Checkpoint ${cp.name} has ${files.length} changed files: ${files.slice(0, 5).join(', ')}${files.length > 5 ? '...' : ''}`);

              // Parse checkpoint message to extract timestamp
              const match = cp.message.match(/^fspec-checkpoint:[^:]+:[^:]+:([^:]+)$/);
              const timestamp = match ? new Date(parseInt(match[1])).toISOString() : new Date().toISOString();
              logger.info(`[${componentId.current}] Checkpoint ${cp.name} timestamp: ${timestamp}`);

              allCheckpoints.push({
                name: cp.name,
                workUnitId,
                timestamp,
                stashRef: ref,
                isAutomatic: cp.name.includes('-auto-'),
                files,
                fileCount: files.length,
              });
              logger.info(`[${componentId.current}] Successfully loaded checkpoint ${cp.name}`);
            } catch (error) {
              // Skip checkpoints that can't be loaded
              logger.error(`[${componentId.current}] Failed to load checkpoint ${cp.name}: ${error}`);
            }
          }
        }

        logger.info(`[${componentId.current}] Loaded ${allCheckpoints.length} total checkpoints`);
        setCheckpoints(allCheckpoints);
      } catch (error) {
        logger.error(`[${componentId.current}] Failed to load checkpoints: ${error}`);
        setCheckpoints([]);
      } finally {
        setIsLoadingCheckpoints(false);
        logger.info(`[${componentId.current}] Checkpoint loading complete`);
      }
    };

    void loadAllCheckpoints();
  }, [cwd]);

  // Sort checkpoints by timestamp (most recent first) and limit to 200
  const sortedCheckpoints = useMemo(() => {
    logger.info(`[${componentId.current}] Sorting ${checkpoints.length} checkpoints by timestamp`);
    const sorted = [...checkpoints].sort((a, b) => {
      return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
    });
    // Limit to 200 most recent checkpoints for performance
    const limited = sorted.slice(0, 200);
    logger.info(`[${componentId.current}] Limited to ${limited.length} most recent checkpoints: ${limited.slice(0, 10).map(c => c.name).join(', ')}${limited.length > 10 ? '...' : ''}`);
    return limited;
  }, [checkpoints]);

  // Get current checkpoint and files
  const currentCheckpoint = sortedCheckpoints[selectedCheckpointIndex];
  const files: FileItem[] = useMemo(() => {
    if (!currentCheckpoint) {
      logger.info(`[${componentId.current}] No current checkpoint selected (index: ${selectedCheckpointIndex})`);
      return [];
    }
    logger.info(`[${componentId.current}] Current checkpoint: ${currentCheckpoint.name} with ${currentCheckpoint.files.length} files`);
    logger.info(`[${componentId.current}] Files: ${currentCheckpoint.files.slice(0, 10).join(', ')}${currentCheckpoint.files.length > 10 ? '...' : ''}`);
    return currentCheckpoint.files.map(f => ({ path: f, status: 'checkpoint' as const }));
  }, [currentCheckpoint, selectedCheckpointIndex]);

  // Initialize worker thread on mount
  useEffect(() => {
    const workerPath = join(process.cwd(), 'dist', 'git', 'diff-worker.js');
    logger.info(`[${componentId.current}] Initializing worker thread at: ${workerPath}`);

    try {
      workerRef.current = new Worker(workerPath);
      logger.info(`[${componentId.current}] Worker thread initialized successfully`);
    } catch (error) {
      logger.error(`[${componentId.current}] Failed to initialize worker: ${error}`);
    }

    return () => {
      if (workerRef.current) {
        logger.info(`[${componentId.current}] Terminating worker thread`);
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  // Load git diff when selected file changes using worker thread
  useEffect(() => {
    const selectedFile = files[selectedFileIndex];

    if (!selectedFile) {
      logger.info(`[${componentId.current}] No selected file, clearing diff`);
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    if (!workerRef.current) {
      logger.error(`[${componentId.current}] Worker not initialized, cannot load diff`);
      setDiffContent('Worker thread not available');
      setIsLoadingDiff(false);
      return;
    }

    logger.info(`[${componentId.current}] Loading diff for file: ${selectedFile.path}`);
    setIsLoadingDiff(true);

    const startTime = Date.now();
    const requestId = `${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;

    // Set up message handler for this request
    const messageHandler = (response: { id: string; diff?: string; error?: string }) => {
      const duration = Date.now() - startTime;

      // Ignore responses from cancelled requests
      if (response.id !== pendingRequestId.current) {
        logger.info(`[${componentId.current}] Received stale response (id=${response.id}), ignoring`);
        return;
      }

      if (response.error) {
        logger.error(`[${componentId.current}] Worker error after ${duration}ms: ${response.error}`);
        setDiffContent('Error loading diff');
      } else {
        const diffLength = response.diff?.length || 0;
        logger.info(`[${componentId.current}] Diff loaded successfully in ${duration}ms (${diffLength} chars)`);

        // Truncate large diffs to prevent UX hangs
        const MAX_DIFF_SIZE = 100000; // 100KB max
        let finalDiff = response.diff || 'No changes to display';
        if (diffLength > MAX_DIFF_SIZE) {
          const truncatedDiff = response.diff!.substring(0, MAX_DIFF_SIZE);
          const linesShown = truncatedDiff.split('\n').length;
          const totalLines = response.diff!.split('\n').length;
          finalDiff = truncatedDiff + `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${diffLength} chars)`;
          logger.info(`[${componentId.current}] Diff truncated from ${diffLength} to ${MAX_DIFF_SIZE} chars`);
        }

        setDiffContent(finalDiff);
      }

      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);

    // Send request to worker with checkpointRef to compare checkpoint vs HEAD
    logger.info(`[${componentId.current}] Sending request to worker (id=${requestId})`);
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
      checkpointRef: currentCheckpoint.stashRef, // Compare checkpoint file vs HEAD
    });

    // Cleanup function to cancel pending requests
    return () => {
      logger.info(`[${componentId.current}] CLEANUP called (cancelling request ${requestId})`);
      pendingRequestId.current = null;
      worker.off('message', messageHandler);
    };
  }, [selectedFileIndex, files, cwd, currentCheckpoint]);

  // Parse diff content into structured DiffLine objects
  const diffLines: DiffLine[] = useMemo(() => {
    if (isLoadingDiff) {
      return [{ content: 'Loading diff...', type: 'context', changeGroup: null }];
    }
    if (!diffContent) {
      return [];
    }
    return parseDiff(diffContent);
  }, [diffContent, isLoadingDiff]);

  // Handle keyboard input
  useInput((input, key) => {
    if (key.escape) {
      onExit();
      return;
    }

    // Tab key to cycle through three panes
    if (key.tab) {
      setFocusedPane(prev => {
        if (prev === 'checkpoints') return 'files';
        if (prev === 'files') return 'diff';
        return 'checkpoints';
      });
      return;
    }

    // Arrow key navigation handled by VirtualList when focused
  });

  // Loading state
  if (isLoadingCheckpoints) {
    return (
      <Box flexDirection="column" flexGrow={1}>
        <Box flexDirection="row" flexGrow={1}>
          <Box flexDirection="column" minWidth={leftColumnMinWidth} flexBasis="25%" flexShrink={1}>
            <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single" borderColor="cyan">
              <Text wrap="truncate">Loading checkpoints...</Text>
            </Box>
            <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single">
              <Text wrap="truncate">-</Text>
            </Box>
          </Box>
          <Box flexDirection="column" flexGrow={1} borderStyle="single">
            <Text wrap="truncate">-</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Empty state
  if (sortedCheckpoints.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1}>
        <Box flexDirection="row" flexGrow={1}>
          {/* Left column: Checkpoint list (top) + File list (bottom) */}
          <Box flexDirection="column" minWidth={leftColumnMinWidth} flexBasis="25%" flexShrink={1}>
            {/* Checkpoint list pane (top-left) */}
            <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single" borderColor="cyan">
              <Text wrap="truncate">No checkpoints available</Text>
            </Box>
            {/* File list pane (bottom-left) */}
            <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single">
              <Text wrap="truncate">No files</Text>
            </Box>
          </Box>

          {/* Right side: Diff pane */}
          <Box flexDirection="column" flexGrow={1} borderStyle="single">
            <Text wrap="truncate">Select a checkpoint to view files</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render checkpoint item with compact name
  const renderCheckpointItem = (checkpoint: Checkpoint, index: number, isSelected: boolean): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';

    // Extract compact name from checkpoint name (e.g., "TUI-001-auto-testing" -> "TUI-001 - TESTING")
    const parts = checkpoint.name.split('-auto-');
    const workUnit = parts[0]; // e.g., "TUI-001"
    const phase = parts[1] ? parts[1].toUpperCase() : 'UNKNOWN'; // e.g., "TESTING"
    const compactName = `${workUnit} - ${phase}`;
    const displayName = `${compactName} (${checkpoint.fileCount} ${checkpoint.fileCount === 1 ? 'file' : 'files'})`;

    return (
      <Box width="100%">
        <Text color={isSelected ? 'cyan' : 'white'} wrap="truncate">
          {indicator} {displayName}
        </Text>
      </Box>
    );
  };

  // Render diff line with syntax highlighting
  const renderDiffLine = (line: DiffLine, index: number, isSelected: boolean): React.ReactNode => {
    let textColor: 'white' | 'cyan' = 'white';
    let backgroundColor: 'red' | 'green' | undefined;

    // Determine colors based on line type
    if (line.type === 'hunk') {
      textColor = 'cyan';
    } else if (line.type === 'removed') {
      textColor = 'white';
      backgroundColor = 'red';
    } else if (line.type === 'added') {
      textColor = 'white';
      backgroundColor = 'green';
    }

    // Apply selection styling if focused
    const selectionColor = isSelected && focusedPane === 'diff' ? 'cyan' : textColor;
    const selectionInverse = isSelected && focusedPane === 'diff';

    return (
      <Box width="100%">
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
      {/* Header */}
      <Box>
        <Text bold>
          Checkpoints: {sortedCheckpoints.length} available
        </Text>
      </Box>

      {/* Three-pane layout: Left column (checkpoint list + file list) + Diff pane (right) */}
      <Box flexDirection="row" flexGrow={1}>
        {/* Left column: Checkpoint list (top) + File list (bottom) stacked vertically */}
        <Box
          flexDirection="column"
          minWidth={leftColumnMinWidth}
          flexBasis="25%"
          flexShrink={1}
        >
          {/* Checkpoint list pane (top-left) - flexGrow 1 for 50/50 split */}
          <Box
            flexDirection="column"
            flexGrow={1}
            flexBasis={0}
            borderStyle="single"
            borderColor={focusedPane === 'checkpoints' ? 'cyan' : 'gray'}
          >
            <Box flexGrow={1}>
              <VirtualList
                items={sortedCheckpoints}
                renderItem={renderCheckpointItem}
                showScrollbar={focusedPane === 'checkpoints'}
                isFocused={focusedPane === 'checkpoints'}
                onFocus={(checkpoint, index) => {
                  logger.info(`[${componentId.current}] Checkpoint selection changed: index=${index}, name=${checkpoint.name}`);
                  setSelectedCheckpointIndex(index);
                  setSelectedFileIndex(0); // Reset file selection when checkpoint changes
                  logger.info(`[${componentId.current}] Reset file selection to 0`);
                }}
              />
            </Box>
          </Box>

          {/* File list pane (bottom-left) - flexGrow 1 for 50/50 split */}
          <Box
            flexDirection="column"
            flexGrow={1}
            flexBasis={0}
            borderStyle="single"
            borderColor={focusedPane === 'files' ? 'cyan' : 'gray'}
          >
            <Box flexGrow={1}>
              <VirtualList
                items={files}
                renderItem={(file, index, isSelected) => {
                  const indicator = isSelected ? '>' : ' ';
                  return (
                    <Box width="100%">
                      <Text color={isSelected ? 'cyan' : 'white'} wrap="truncate">
                        {indicator} {file.path}
                      </Text>
                    </Box>
                  );
                }}
                showScrollbar={focusedPane === 'files'}
                isFocused={focusedPane === 'files'}
                onFocus={(file, index) => {
                  logger.info(`[${componentId.current}] File selection changed: index=${index}, path=${file.path}`);
                  setSelectedFileIndex(index);
                }}
              />
            </Box>
          </Box>
        </Box>

        {/* Right side: Diff pane only (grows to fill remaining space) */}
        <Box
          flexDirection="column"
          flexGrow={1}
          borderStyle="single"
          borderColor={focusedPane === 'diff' ? 'cyan' : 'gray'}
        >
          <Box flexGrow={1}>
            <VirtualList
              items={diffLines}
              renderItem={renderDiffLine}
              showScrollbar={focusedPane === 'diff'}
              isFocused={focusedPane === 'diff'}
            />
          </Box>
        </Box>
      </Box>

      {/* Footer */}
      <Box>
        <Text dimColor>
          ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll
        </Text>
      </Box>
    </Box>
  );
};
