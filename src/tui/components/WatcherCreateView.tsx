/**
 * WatcherCreateView - Full-screen form view for creating a new watcher session
 *
 * WATCH-009: Watcher Creation Dialog UI
 *
 * This component follows the architecture design from WATCH-001:
 * - Full-screen form overlay (similar to /resume and /search modes)
 * - Role Name input field (required)
 * - Authority selector (Peer | Supervisor)
 * - Model selector (populated from available provider models)
 * - Brief textarea (watching instructions)
 *
 * Keyboard Navigation:
 * - Tab: Cycle focus between fields
 * - ←/→: Toggle Authority when focused
 * - ↑/↓: Navigate Model selection when focused
 * - Enter: Create watcher (when valid)
 * - Esc: Cancel and return to overlay
 */

import React, { useState, useCallback } from 'react';
import { Box, Text, useInput } from 'ink';

// Focus field type for cycling
type FocusField = 'name' | 'authority' | 'model' | 'brief' | 'create';

// Focus order constant - single source of truth for tab cycling
const FOCUS_ORDER: FocusField[] = ['name', 'authority', 'model', 'brief', 'create'];

interface WatcherCreateViewProps {
  /** Current model of the parent session (default for new watcher) */
  currentModel: string;
  /** Available models to choose from */
  availableModels: string[];
  /** Terminal dimensions */
  terminalWidth: number;
  terminalHeight: number;
  /** Callback when watcher is created */
  onCreate: (name: string, authority: 'peer' | 'supervisor', model: string, brief: string) => void;
  /** Callback when creation is cancelled */
  onCancel: () => void;
}

export function WatcherCreateView({
  currentModel,
  availableModels,
  terminalWidth,
  terminalHeight,
  onCreate,
  onCancel,
}: WatcherCreateViewProps): React.ReactElement {
  // Form state
  const [name, setName] = useState('');
  const [authority, setAuthority] = useState<'peer' | 'supervisor'>('peer');
  const [selectedModelIndex, setSelectedModelIndex] = useState(() => {
    const idx = availableModels.indexOf(currentModel);
    return idx >= 0 ? idx : 0;
  });
  const [brief, setBrief] = useState('');
  const [focusField, setFocusField] = useState<FocusField>('name');

  // Focus cycling logic using FOCUS_ORDER constant (DRY)
  const cycleFocusForward = useCallback(() => {
    const currentIndex = FOCUS_ORDER.indexOf(focusField);
    setFocusField(FOCUS_ORDER[(currentIndex + 1) % FOCUS_ORDER.length]);
  }, [focusField]);

  const cycleFocusBackward = useCallback(() => {
    const currentIndex = FOCUS_ORDER.indexOf(focusField);
    setFocusField(FOCUS_ORDER[(currentIndex - 1 + FOCUS_ORDER.length) % FOCUS_ORDER.length]);
  }, [focusField]);

  // Handle create action
  const handleCreate = useCallback(() => {
    if (!name.trim()) return; // Name is required
    const selectedModel = availableModels[selectedModelIndex] || currentModel;
    onCreate(name.trim(), authority, selectedModel, brief.trim());
  }, [name, authority, selectedModelIndex, availableModels, currentModel, brief, onCreate]);

  // Keyboard input handling
  useInput((input, key) => {
    // Escape always cancels
    if (key.escape) {
      onCancel();
      return;
    }

    // Tab cycles focus forward, Shift+Tab backward
    if (key.tab) {
      if (key.shift) {
        cycleFocusBackward();
      } else {
        cycleFocusForward();
      }
      return;
    }

    // Enter creates watcher (from any field, if name is valid)
    if (key.return) {
      handleCreate();
      return;
    }

    // Field-specific input handling
    switch (focusField) {
      case 'name':
        // Text input for name field
        if (key.backspace || key.delete) {
          setName(prev => prev.slice(0, -1));
        } else if (input && !key.ctrl && !key.meta) {
          setName(prev => prev + input);
        }
        break;

      case 'authority':
        // Left/Right toggles authority
        if (key.leftArrow || key.rightArrow) {
          setAuthority(prev => (prev === 'peer' ? 'supervisor' : 'peer'));
        }
        break;

      case 'model':
        // Up/Down navigates model selection
        if (key.upArrow) {
          setSelectedModelIndex(prev => Math.max(0, prev - 1));
        } else if (key.downArrow) {
          setSelectedModelIndex(prev => Math.min(availableModels.length - 1, prev + 1));
        }
        break;

      case 'brief':
        // Text input for brief field (multiline - Enter adds newline when focused here)
        if (key.backspace || key.delete) {
          setBrief(prev => prev.slice(0, -1));
        } else if (input && !key.ctrl && !key.meta) {
          setBrief(prev => prev + input);
        }
        break;

      case 'create':
        // Enter on create button triggers creation
        // (already handled above)
        break;
    }
  });

  const isNameValid = name.trim().length > 0;
  const selectedModel = availableModels[selectedModelIndex] || currentModel;

  return (
    <Box
      position="absolute"
      flexDirection="column"
      width={terminalWidth}
      height={terminalHeight}
    >
      <Box
        flexDirection="column"
        flexGrow={1}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2} flexGrow={1}>
          {/* Header */}
          <Box marginBottom={1} borderStyle="single" borderBottom borderLeft={false} borderRight={false} borderTop={false}>
            <Text bold color="magenta">Create New Watcher</Text>
          </Box>

          {/* Role Name field */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'name' ? 'cyan' : 'white'}>
              Role Name{!isNameValid && focusField !== 'name' ? ' (required)' : ''}:
            </Text>
            <Box>
              <Text
                backgroundColor={focusField === 'name' ? 'blue' : undefined}
                color={focusField === 'name' ? 'white' : 'gray'}
              >
                {name || (focusField === 'name' ? '' : '(empty)')}
                {focusField === 'name' ? '▌' : ''}
              </Text>
            </Box>
            {!isNameValid && (
              <Text color="red" dimColor>
                ⚠ Role name is required
              </Text>
            )}
          </Box>

          {/* Authority selector */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'authority' ? 'cyan' : 'white'}>
              Authority:
            </Text>
            <Box>
              <Text
                backgroundColor={focusField === 'authority' && authority === 'peer' ? 'blue' : undefined}
                color={authority === 'peer' ? 'green' : 'gray'}
              >
                [{authority === 'peer' ? '●' : ' '}] Peer
              </Text>
              <Text> </Text>
              <Text
                backgroundColor={focusField === 'authority' && authority === 'supervisor' ? 'blue' : undefined}
                color={authority === 'supervisor' ? 'yellow' : 'gray'}
              >
                [{authority === 'supervisor' ? '●' : ' '}] Supervisor
              </Text>
              {focusField === 'authority' && (
                <Text dimColor> (←/→ to toggle)</Text>
              )}
            </Box>
          </Box>

          {/* Model selector */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'model' ? 'cyan' : 'white'}>
              Model:
            </Text>
            <Box>
              <Text
                backgroundColor={focusField === 'model' ? 'blue' : undefined}
                color={focusField === 'model' ? 'white' : 'gray'}
              >
                {selectedModel}
              </Text>
              {focusField === 'model' && (
                <Text dimColor> (↑/↓ to change, {selectedModelIndex + 1}/{availableModels.length})</Text>
              )}
            </Box>
          </Box>

          {/* Brief textarea */}
          <Box marginBottom={1} flexDirection="column" flexGrow={1}>
            <Text color={focusField === 'brief' ? 'cyan' : 'white'}>
              Brief (watching instructions):
            </Text>
            <Box
              borderStyle="single"
              borderColor={focusField === 'brief' ? 'cyan' : 'gray'}
              padding={1}
              flexGrow={1}
            >
              <Text color={focusField === 'brief' ? 'white' : 'gray'} wrap="wrap">
                {brief || (focusField === 'brief' ? '' : '(optional - describe what to watch for)')}
                {focusField === 'brief' ? '▌' : ''}
              </Text>
            </Box>
          </Box>

          {/* Create button */}
          <Box marginTop={1}>
            <Text
              backgroundColor={focusField === 'create' ? (isNameValid ? 'green' : 'red') : undefined}
              color={focusField === 'create' ? 'white' : (isNameValid ? 'green' : 'red')}
              bold={focusField === 'create'}
            >
              {focusField === 'create' ? '[ Create Watcher ]' : '  Create Watcher  '}
            </Text>
            {!isNameValid && (
              <Text color="red" dimColor> (name required)</Text>
            )}
          </Box>

          {/* Footer with keyboard hints */}
          <Box marginTop={1} borderStyle="single" borderTop borderBottom={false} borderLeft={false} borderRight={false}>
            <Text dimColor>
              Tab: Next Field | Shift+Tab: Previous | Enter: Create | Esc: Cancel
            </Text>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
