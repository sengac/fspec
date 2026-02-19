/**
 * SessionManagementPanel.tsx - Panel for managing isolated session worktrees
 *
 * GIT-029: TUI integration for isolated sessions
 *
 * This panel displays completed isolated sessions (those with worktrees)
 * and allows users to:
 * - View session status (pending_merge, clean, orphaned)
 * - See diff summary (files changed count)
 * - Merge session changes to main worktree
 * - Discard session changes without applying
 * - Prune all orphaned sessions
 *
 * Status colors:
 * - pending_merge (yellow): Session has changes waiting to be merged
 * - clean (green): Session completed with no changes
 * - orphaned (red): Worktree exists but session record is missing
 * - active (blue): Session is currently running
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from '../../components/Dialog';
import { useInputCompat, InputPriority } from '../input/index';
import {
  listSessionWorktrees,
  inspectSessionChanges,
  mergeSessionChanges,
  discardSessionChanges,
  pruneOrphanedSessions,
} from '../services/sessionService';
import { sessionManagerList } from '@sengac/codelet-napi';
import type { SessionInfoJs, SessionResultJs } from '@sengac/codelet-napi';

export interface SessionManagementPanelProps {
  /** Path to the git repository */
  repoPath: string;
  /** Callback when panel should close */
  onClose: () => void;
  /** Whether the panel is active for input handling */
  isActive?: boolean;
}

interface SessionWithDetails extends SessionInfoJs {
  details?: SessionResultJs;
}

type ConfirmAction = {
  type: 'merge' | 'discard' | 'prune';
  sessionId?: string;
};

/**
 * Get status badge color based on session status
 */
function getStatusColor(status: string): string {
  switch (status) {
    case 'pendingmerge':
      return 'yellow';
    case 'clean':
      return 'green';
    case 'orphaned':
      return 'red';
    case 'active':
      return 'blue';
    default:
      return 'gray';
  }
}

/**
 * Format status for display (convert from internal format)
 */
function formatStatus(status: string): string {
  switch (status) {
    case 'pendingmerge':
      return 'pending_merge';
    default:
      return status;
  }
}

/**
 * SessionManagementPanel - Manage completed isolated sessions
 */
export const SessionManagementPanel: React.FC<SessionManagementPanelProps> = ({
  repoPath,
  onClose,
  isActive = true,
}) => {
  const [sessions, setSessions] = useState<SessionWithDetails[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Load sessions on mount
  const loadSessions = useCallback(() => {
    setLoading(true);
    try {
      // Get active session IDs from background sessions
      const backgroundSessions = sessionManagerList();
      const activeIds = backgroundSessions.map(s => s.id);

      // List all session worktrees
      const sessionList = listSessionWorktrees(repoPath, activeIds);
      
      // Load details for each session
      const sessionsWithDetails: SessionWithDetails[] = sessionList.map(session => {
        try {
          const details = inspectSessionChanges(repoPath, session.sessionId);
          return { ...session, details };
        } catch {
          return session;
        }
      });

      setSessions(sessionsWithDetails);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setMessage(`Error loading sessions: ${errorMsg}`);
    } finally {
      setLoading(false);
    }
  }, [repoPath]);

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Handle merge action
  const handleMerge = useCallback((sessionId: string) => {
    try {
      const result = mergeSessionChanges(repoPath, sessionId);
      setMessage(`Merged session ${sessionId.slice(0, 8)}... (${result.filesModified.length} modified, ${result.filesAdded.length} added, ${result.filesDeleted.length} deleted)`);
      loadSessions(); // Refresh list
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setMessage(`Merge failed: ${errorMsg}`);
    }
  }, [repoPath, loadSessions]);

  // Handle discard action
  const handleDiscard = useCallback((sessionId: string) => {
    try {
      const result = discardSessionChanges(repoPath, sessionId);
      setMessage(`Discarded session ${sessionId.slice(0, 8)}... (${result.filesDiscarded} files)`);
      loadSessions(); // Refresh list
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setMessage(`Discard failed: ${errorMsg}`);
    }
  }, [repoPath, loadSessions]);

  // Handle prune orphaned action
  const handlePruneOrphaned = useCallback(() => {
    try {
      const backgroundSessions = sessionManagerList();
      const activeIds = backgroundSessions.map(s => s.id);
      const result = pruneOrphanedSessions(repoPath, activeIds);
      setMessage(`Pruned ${result.count} orphaned session(s)`);
      loadSessions(); // Refresh list
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setMessage(`Prune failed: ${errorMsg}`);
    }
  }, [repoPath, loadSessions]);

  // Input handling
  useInputCompat({
    id: 'session-management-panel',
    priority: InputPriority.DIALOG,
    isActive: isActive && !confirmAction,
    handler: (_input, key) => {
      if (key.escape) {
        onClose();
        return true;
      }

      if (sessions.length === 0) {
        return false;
      }

      if (key.upArrow) {
        setSelectedIndex(prev => Math.max(0, prev - 1));
        return true;
      }
      if (key.downArrow) {
        setSelectedIndex(prev => Math.min(sessions.length - 1, prev + 1));
        return true;
      }

      const selectedSession = sessions[selectedIndex];
      if (!selectedSession) {
        return false;
      }

      // M for merge
      if (_input === 'm' || _input === 'M') {
        if (selectedSession.status === 'pendingmerge' || selectedSession.status === 'clean') {
          setConfirmAction({ type: 'merge', sessionId: selectedSession.sessionId });
        }
        return true;
      }

      // D for discard
      if (_input === 'd' || _input === 'D') {
        if (selectedSession.status !== 'active') {
          setConfirmAction({ type: 'discard', sessionId: selectedSession.sessionId });
        }
        return true;
      }

      // P for prune orphaned
      if (_input === 'p' || _input === 'P') {
        const hasOrphaned = sessions.some(s => s.status === 'orphaned');
        if (hasOrphaned) {
          setConfirmAction({ type: 'prune' });
        }
        return true;
      }

      // R for refresh
      if (_input === 'r' || _input === 'R') {
        loadSessions();
        return true;
      }

      return false;
    },
  });

  // Confirmation dialog input handling
  useInputCompat({
    id: 'session-management-confirm',
    priority: InputPriority.CRITICAL,
    isActive: isActive && confirmAction !== null,
    handler: (_input, key) => {
      if (key.escape || _input === 'n' || _input === 'N') {
        setConfirmAction(null);
        return true;
      }

      if (_input === 'y' || _input === 'Y' || key.return) {
        if (confirmAction) {
          if (confirmAction.type === 'merge' && confirmAction.sessionId) {
            handleMerge(confirmAction.sessionId);
          } else if (confirmAction.type === 'discard' && confirmAction.sessionId) {
            handleDiscard(confirmAction.sessionId);
          } else if (confirmAction.type === 'prune') {
            handlePruneOrphaned();
          }
        }
        setConfirmAction(null);
        return true;
      }

      return false;
    },
  });

  // Clear message after delay
  useEffect(() => {
    if (message) {
      const timer = setTimeout(() => setMessage(null), 5000);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [message]);

  const hasOrphaned = sessions.some(s => s.status === 'orphaned');

  return (
    <Dialog onClose={onClose} borderColor="cyan" width={80}>
      <Text bold>Session Management</Text>
      <Text dimColor>Manage isolated session worktrees</Text>

      {loading ? (
        <Box marginTop={1}>
          <Text dimColor>Loading sessions...</Text>
        </Box>
      ) : sessions.length === 0 ? (
        <Box marginTop={1}>
          <Text dimColor>No isolated sessions found</Text>
        </Box>
      ) : (
        <Box marginTop={1} flexDirection="column">
          {sessions.map((session, index) => {
            const isSelected = index === selectedIndex;
            const statusColor = getStatusColor(session.status);
            const filesChanged = session.filesChanged || 0;

            return (
              <Box key={session.sessionId}>
                <Text
                  backgroundColor={isSelected ? 'blue' : undefined}
                  color={isSelected ? 'white' : undefined}
                >
                  {isSelected ? '▶ ' : '  '}
                </Text>
                <Text color={statusColor} bold>
                  [{formatStatus(session.status)}]
                </Text>
                <Text> </Text>
                <Text>{session.sessionId.slice(0, 8)}...</Text>
                <Text dimColor> ({filesChanged} files changed)</Text>
              </Box>
            );
          })}
        </Box>
      )}

      {/* Selected session details */}
      {sessions[selectedIndex]?.details && (
        <Box marginTop={1} flexDirection="column" borderStyle="single" borderColor="gray" paddingX={1}>
          <Text bold>Changes:</Text>
          <Text dimColor>
            Modified: {sessions[selectedIndex].details?.filesChanged?.join(', ') || 'none'}
          </Text>
          <Text dimColor>
            Added: {sessions[selectedIndex].details?.filesAdded?.join(', ') || 'none'}
          </Text>
          <Text dimColor>
            Deleted: {sessions[selectedIndex].details?.filesDeleted?.join(', ') || 'none'}
          </Text>
        </Box>
      )}

      {/* Message display */}
      {message && (
        <Box marginTop={1}>
          <Text color="cyan">{message}</Text>
        </Box>
      )}

      {/* Help */}
      <Box marginTop={1} justifyContent="center">
        <Text dimColor>
          ↑↓ Navigate | M Merge | D Discard{hasOrphaned ? ' | P Prune Orphaned' : ''} | R Refresh | Esc Close
        </Text>
      </Box>

      {/* Confirmation Dialog */}
      {confirmAction && (
        <Box
          position="absolute"
          marginTop={-5}
          flexDirection="column"
          borderStyle="round"
          borderColor="yellow"
          paddingX={2}
          paddingY={1}
        >
          <Text bold color="yellow">
            {confirmAction.type === 'merge' && 'Merge this session?'}
            {confirmAction.type === 'discard' && 'Discard this session?'}
            {confirmAction.type === 'prune' && 'Prune all orphaned sessions?'}
          </Text>
          <Text dimColor>
            {confirmAction.type === 'merge' && 'Changes will be applied to the main worktree.'}
            {confirmAction.type === 'discard' && 'All changes will be lost.'}
            {confirmAction.type === 'prune' && 'Orphaned worktrees will be removed.'}
          </Text>
          <Box marginTop={1}>
            <Text>Y/Enter to confirm, N/Esc to cancel</Text>
          </Box>
        </Box>
      )}
    </Dialog>
  );
};
