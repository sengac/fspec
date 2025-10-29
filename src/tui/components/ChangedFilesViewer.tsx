/**
 * ChangedFilesViewer Component - dual-pane viewer for changed files and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 */

import React, { useState, useEffect, useMemo, useRef } from 'react';
import { Box, Text, useInput } from 'ink';
import { VirtualList } from './VirtualList';
import { useFspecStore } from '../store/fspecStore';
import { logger } from '../../utils/logger';
import { Worker } from 'worker_threads';
import { join } from 'path';

interface ChangedFilesViewerProps {
  stagedFiles: string[];
  unstagedFiles: string[];
  onExit: () => void;
}

const ChangedFilesViewerComponent: React.FC<ChangedFilesViewerProps> = ({
  stagedFiles,
  unstagedFiles,
  onExit,
}) => {
  const [focusedPane, setFocusedPane] = useState<'files' | 'diff'>('files');
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);

  const cwd = useFspecStore(state => state.cwd);

  // Debug counters
  const renderCount = useRef(0);
  const effectRunCount = useRef(0);
  const componentId = useRef(`ChangedFilesViewer-${Date.now()}`);

  // Track what caused the render
  const prevPropsRef = useRef<{
    selectedFileIndex: number;
    stagedFiles: string[];
    unstagedFiles: string[];
    focusedPane: 'files' | 'diff';
    diffContent: string;
    isLoadingDiff: boolean;
  }>();

  // Track component renders and what changed
  renderCount.current++;

  const renderCauses: string[] = [];
  if (prevPropsRef.current) {
    if (prevPropsRef.current.selectedFileIndex !== selectedFileIndex) {
      renderCauses.push(`selectedFileIndex: ${prevPropsRef.current.selectedFileIndex} → ${selectedFileIndex}`);
    }
    if (prevPropsRef.current.stagedFiles !== stagedFiles) {
      renderCauses.push(`stagedFiles: [${prevPropsRef.current.stagedFiles.length} items] → [${stagedFiles.length} items]`);
    }
    if (prevPropsRef.current.unstagedFiles !== unstagedFiles) {
      renderCauses.push(`unstagedFiles: [${prevPropsRef.current.unstagedFiles.length} items] → [${unstagedFiles.length} items]`);
    }
    if (prevPropsRef.current.focusedPane !== focusedPane) {
      renderCauses.push(`focusedPane: ${prevPropsRef.current.focusedPane} → ${focusedPane}`);
    }
    if (prevPropsRef.current.diffContent !== diffContent) {
      const oldLen = prevPropsRef.current.diffContent.length;
      const newLen = diffContent.length;
      renderCauses.push(`diffContent: [${oldLen} chars] → [${newLen} chars]`);
    }
    if (prevPropsRef.current.isLoadingDiff !== isLoadingDiff) {
      renderCauses.push(`isLoadingDiff: ${prevPropsRef.current.isLoadingDiff} → ${isLoadingDiff}`);
    }
  }

  const causeMsg = renderCauses.length > 0 ? ` | CAUSES: ${renderCauses.join(', ')}` : ' | INITIAL RENDER';
  logger.info(`[${componentId.current}] RENDER #${renderCount.current}${causeMsg}`);

  // Store current values for next render
  prevPropsRef.current = {
    selectedFileIndex,
    stagedFiles,
    unstagedFiles,
    focusedPane,
    diffContent,
    isLoadingDiff,
  };

  // File list minimum width in characters
  const fileListMinWidth = 30;

  // Combine staged and unstaged files with status indicators
  // FIX: Memoize allFiles to prevent object reference instability
  const allFiles = useMemo(() => {
    logger.info(`[${componentId.current}] useMemo(allFiles) - Recomputing allFiles array (stagedFiles.length=${stagedFiles.length}, unstagedFiles.length=${unstagedFiles.length})`);
    return [
      ...stagedFiles.map(f => ({ path: f, status: 'staged' as const })),
      ...unstagedFiles.map(f => ({ path: f, status: 'unstaged' as const })),
    ];
  }, [stagedFiles, unstagedFiles]);

  // Worker thread reference
  const workerRef = useRef<Worker | null>(null);
  const pendingRequestId = useRef<string | null>(null);

  // Initialize worker thread on mount
  useEffect(() => {
    // Get the worker file path - need to resolve relative to dist/
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

  // Load real git diff when selected file changes using worker thread
  // FIX: Depend on selectedFileIndex primitive, not selectedFile object
  useEffect(() => {
    effectRunCount.current++;
    const effectId = effectRunCount.current;

    logger.info(`[${componentId.current}] useEffect RUN #${effectId} - TRIGGERED with selectedFileIndex=${selectedFileIndex}, stagedFiles.length=${stagedFiles.length}, unstagedFiles.length=${unstagedFiles.length}, cwd=${cwd}`);

    const selectedFile = allFiles[selectedFileIndex];

    if (!selectedFile) {
      logger.info(`[${componentId.current}] useEffect #${effectId} - No selected file, clearing diff`);
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    if (!workerRef.current) {
      logger.error(`[${componentId.current}] useEffect #${effectId} - Worker not initialized, cannot load diff`);
      setDiffContent('Worker thread not available');
      setIsLoadingDiff(false);
      return;
    }

    logger.info(`[${componentId.current}] useEffect #${effectId} - Loading diff for file: ${selectedFile.path}`);
    setIsLoadingDiff(true);

    const startTime = Date.now();
    const requestId = `${effectId}-${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;

    // Set up message handler for this request
    const messageHandler = (response: { id: string; diff?: string; error?: string }) => {
      const duration = Date.now() - startTime;

      // Ignore responses from cancelled requests
      if (response.id !== pendingRequestId.current) {
        logger.info(`[${componentId.current}] useEffect #${effectId} - Received stale response (id=${response.id}), ignoring`);
        return;
      }

      if (response.error) {
        logger.error(`[${componentId.current}] useEffect #${effectId} - Worker error after ${duration}ms: ${response.error}`);
        setDiffContent('Error loading diff');
      } else {
        const diffLength = response.diff?.length || 0;
        logger.info(`[${componentId.current}] useEffect #${effectId} - Diff loaded successfully in ${duration}ms (${diffLength} chars)`);

        // FIX: Truncate large diffs to prevent UX hangs
        const MAX_DIFF_SIZE = 100000; // 100KB max
        let finalDiff = response.diff || 'No changes to display';
        if (diffLength > MAX_DIFF_SIZE) {
          const truncatedDiff = response.diff!.substring(0, MAX_DIFF_SIZE);
          const linesShown = truncatedDiff.split('\n').length;
          const totalLines = response.diff!.split('\n').length;
          finalDiff = truncatedDiff + `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${diffLength} chars)`;
          logger.info(`[${componentId.current}] useEffect #${effectId} - Diff truncated from ${diffLength} to ${MAX_DIFF_SIZE} chars`);
        }

        setDiffContent(finalDiff);
      }

      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);

    // Send request to worker
    logger.info(`[${componentId.current}] useEffect #${effectId} - Sending request to worker (id=${requestId})`);
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
    });

    // FIX: Cleanup function to cancel pending requests
    return () => {
      logger.info(`[${componentId.current}] useEffect #${effectId} - CLEANUP called (cancelling request ${requestId})`);
      pendingRequestId.current = null;
      worker.off('message', messageHandler);
    };
  }, [selectedFileIndex, stagedFiles, unstagedFiles, cwd, allFiles]);

  // Parse diff content into lines, show loading state
  const diffLines = isLoadingDiff
    ? ['Loading diff...']
    : (diffContent ? diffContent.split('\n') : []);

  // Handle keyboard input
  useInput((input, key) => {
    if (key.escape) {
      onExit();
      return;
    }

    // Tab key to switch focus between panes
    if (key.tab) {
      setFocusedPane(prev => (prev === 'files' ? 'diff' : 'files'));
      return;
    }

    // Arrow key navigation handled by VirtualList when focused
  });

  // Empty state
  if (allFiles.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1}>
        <Box flexDirection="row" flexGrow={1}>
          {/* File list pane */}
          <Box flexDirection="column" minWidth={fileListMinWidth} flexBasis="25%" flexShrink={1} borderStyle="single" borderColor="cyan">
            <Text>No changed files</Text>
          </Box>

          {/* Diff pane */}
          <Box flexDirection="column" flexGrow={1} borderStyle="single">
            <Text>No changes to display</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render file item with status indicator
  const renderFileItem = (
    file: { path: string; status: 'staged' | 'unstaged' },
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';
    const statusIcon = file.status === 'staged' ? '+' : 'M';
    const statusColor = file.status === 'staged' ? 'green' : 'yellow';

    return (
      <Box width="100%">
        <Text color={isSelected ? 'cyan' : 'white'}>
          {indicator} <Text color={statusColor}>{statusIcon}</Text> {file.path}
        </Text>
      </Box>
    );
  };

  // Render diff line
  const renderDiffLine = (line: string, index: number, isSelected: boolean): React.ReactNode => {
    let color: 'white' | 'green' | 'red' | 'cyan' = 'white';
    if (line.startsWith('+')) {
      color = 'green';
    } else if (line.startsWith('-')) {
      color = 'red';
    } else if (line.startsWith('@@')) {
      color = 'cyan';
    }

    return (
      <Box width="100%">
        <Text color={isSelected && focusedPane === 'diff' ? 'cyan' : color} inverse={isSelected && focusedPane === 'diff'}>
          {line}
        </Text>
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1}>
        {/* Header */}
        <Box>
          <Text bold>
            Changed Files: {stagedFiles.length} staged, {unstagedFiles.length} unstaged
          </Text>
        </Box>

        {/* Dual-pane layout */}
        <Box flexDirection="row" flexGrow={1}>
          {/* File list pane (left, ~25% width with 30 char minimum) */}
          <Box
            flexDirection="column"
            minWidth={fileListMinWidth}
            flexBasis="25%"
            flexShrink={1}
            borderStyle="single"
            borderColor={focusedPane === 'files' ? 'cyan' : 'gray'}
          >
            <Text bold>Files</Text>
            <Box flexGrow={1}>
              <VirtualList
                items={allFiles}
                renderItem={renderFileItem}
                showScrollbar={focusedPane === 'files'}
                isFocused={focusedPane === 'files'}
                onFocus={(file, index) => setSelectedFileIndex(index)}
              />
            </Box>
          </Box>

          {/* Diff pane (right, grows to fill remaining space) */}
          <Box
            flexDirection="column"
            flexGrow={1}
            borderStyle="single"
            borderColor={focusedPane === 'diff' ? 'cyan' : 'gray'}
          >
            <Text bold>Diff</Text>
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

// Export the memoized component
export const ChangedFilesViewer = React.memo(ChangedFilesViewerComponent, (prevProps, nextProps) => {
  // Custom comparison function for React.memo
  // Return true if props are equal (skip re-render), false if different (re-render)

  logger.info(`[ChangedFilesViewer] React.memo comparison function CALLED`)

  // Compare array lengths first (fast check)
  if (prevProps.stagedFiles.length !== nextProps.stagedFiles.length) {
    logger.info(`[ChangedFilesViewer] React.memo: stagedFiles length changed (${prevProps.stagedFiles.length} → ${nextProps.stagedFiles.length})`);
    return false;
  }
  if (prevProps.unstagedFiles.length !== nextProps.unstagedFiles.length) {
    logger.info(`[ChangedFilesViewer] React.memo: unstagedFiles length changed (${prevProps.unstagedFiles.length} → ${nextProps.unstagedFiles.length})`);
    return false;
  }

  // Deep compare array contents
  const stagedEqual = prevProps.stagedFiles.every((file, index) => file === nextProps.stagedFiles[index]);
  const unstagedEqual = prevProps.unstagedFiles.every((file, index) => file === nextProps.unstagedFiles[index]);

  if (!stagedEqual) {
    logger.info(`[ChangedFilesViewer] React.memo: stagedFiles content changed`);
    return false;
  }
  if (!unstagedEqual) {
    logger.info(`[ChangedFilesViewer] React.memo: unstagedFiles content changed`);
    return false;
  }

  // onExit is a function, compare by reference (parent should memoize it)
  if (prevProps.onExit !== nextProps.onExit) {
    logger.info(`[ChangedFilesViewer] React.memo: onExit function changed (parent re-created it)`);
    return false;
  }

  // All props are equal, skip re-render
  logger.info(`[ChangedFilesViewer] React.memo: Props unchanged, SKIPPING RE-RENDER`);
  return true;
});
