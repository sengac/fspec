/**
 * WatcherTemplateForm - Form for creating/editing watcher templates
 *
 * WATCH-023: Watcher Templates and Improved Creation UX
 * Refactored from WatcherCreateView.tsx (WATCH-009, WATCH-021)
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 *
 * Features:
 * - Supports create/edit modes
 * - Pre-populates fields when editing
 * - ↑/↓ arrows navigate between fields (in addition to Tab)
 * - Type-to-filter model selection
 * - Inline authority explanation when focused
 *
 * @see spec/features/watcher-templates.feature
 */

import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { Box, Text } from 'ink';
import type { WatcherTemplate } from '../types/watcherTemplate';
import { useInputCompat, InputPriority } from '../input/index';

type FocusField = 'name' | 'model' | 'authority' | 'brief' | 'autoInject';
const FOCUS_ORDER: FocusField[] = ['name', 'model', 'authority', 'brief', 'autoInject'];

interface WatcherTemplateFormProps {
  mode: 'create' | 'edit';
  template?: WatcherTemplate;
  currentModel: string;
  availableModels: string[];
  terminalWidth: number;
  terminalHeight: number;
  onSave: (name: string, authority: 'peer' | 'supervisor', model: string, brief: string, autoInject: boolean) => void;
  onCancel: () => void;
}

export function WatcherTemplateForm({
  mode,
  template,
  currentModel,
  availableModels,
  terminalWidth,
  terminalHeight,
  onSave,
  onCancel,
}: WatcherTemplateFormProps): React.ReactElement {
  const [name, setName] = useState(template?.name ?? '');
  const [authority, setAuthority] = useState<'peer' | 'supervisor'>(template?.authority ?? 'peer');
  const [modelFilter, setModelFilter] = useState('');
  const [selectedModelIndex, setSelectedModelIndex] = useState(0);
  const [brief, setBrief] = useState(template?.brief ?? '');
  const [autoInject, setAutoInject] = useState(template?.autoInject ?? true);
  const [focusField, setFocusField] = useState<FocusField>('name');

  // Filter models by type-to-filter query
  const filteredModels = useMemo(() => {
    if (!modelFilter.trim()) return availableModels;
    const query = modelFilter.toLowerCase();
    return availableModels.filter(m => m.toLowerCase().includes(query));
  }, [availableModels, modelFilter]);

  // Get currently selected model
  const selectedModel = useMemo(() => {
    if (filteredModels.length === 0) return currentModel;
    return filteredModels[Math.min(selectedModelIndex, filteredModels.length - 1)];
  }, [filteredModels, selectedModelIndex, currentModel]);

  // Initialize model when editing
  useEffect(() => {
    if (mode === 'edit' && template) {
      setName(template.name);
      setAuthority(template.authority);
      setBrief(template.brief);
      setAutoInject(template.autoInject);
      // Set initial filter to show current model
      const modelName = template.modelId.split('/').pop() || '';
      setModelFilter(modelName);
    } else if (mode === 'create') {
      // Default to parent's current model
      const modelName = currentModel.split('/').pop() || '';
      setModelFilter(modelName);
    }
  }, [mode, template, currentModel]);

  // Reset selected index when filter changes
  useEffect(() => {
    setSelectedModelIndex(0);
  }, [modelFilter]);

  const cycleFocusForward = useCallback(() => {
    const i = FOCUS_ORDER.indexOf(focusField);
    setFocusField(FOCUS_ORDER[(i + 1) % FOCUS_ORDER.length]);
  }, [focusField]);

  const cycleFocusBackward = useCallback(() => {
    const i = FOCUS_ORDER.indexOf(focusField);
    setFocusField(FOCUS_ORDER[(i - 1 + FOCUS_ORDER.length) % FOCUS_ORDER.length]);
  }, [focusField]);

  const handleSave = useCallback(() => {
    if (!name.trim()) return;
    onSave(name.trim(), authority, selectedModel, brief.trim(), autoInject);
  }, [name, authority, selectedModel, brief, autoInject, onSave]);

  // Handle keyboard input with CRITICAL priority (full-screen form)
  useInputCompat({
    id: 'watcher-template-form',
    priority: InputPriority.CRITICAL,
    description: 'Watcher template form keyboard navigation',
    handler: (input, key) => {
      if (key.escape) {
        onCancel();
        return true;
      }

      if (key.tab) {
        if (key.shift) {
          cycleFocusBackward();
        } else {
          cycleFocusForward();
        }
        return true;
      }

      if (key.return) {
        handleSave();
        return true;
      }

      // Field-specific handling
      switch (focusField) {
        case 'name':
          if (key.upArrow) { cycleFocusBackward(); return true; }
          if (key.downArrow) { cycleFocusForward(); return true; }
          if (key.backspace || key.delete) {
            setName(p => p.slice(0, -1));
          } else if (input && !key.ctrl && !key.meta) {
            setName(p => p + input);
          }
          break;

        case 'model':
          // Type-to-filter: typing filters the list
          if (key.backspace || key.delete) {
            setModelFilter(p => p.slice(0, -1));
            return true;
          }
          // ↑/↓ navigates filtered list
          if (key.upArrow) {
            setSelectedModelIndex(p => Math.max(0, p - 1));
            return true;
          }
          if (key.downArrow) {
            setSelectedModelIndex(p => Math.min(filteredModels.length - 1, p + 1));
            return true;
          }
          // Accept printable characters for filtering
          if (input && !key.ctrl && !key.meta) {
            setModelFilter(p => p + input);
          }
          break;

        case 'authority':
          if (key.upArrow) { cycleFocusBackward(); return true; }
          if (key.downArrow) { cycleFocusForward(); return true; }
          if (key.leftArrow || key.rightArrow) {
            setAuthority(p => p === 'peer' ? 'supervisor' : 'peer');
          }
          break;

        case 'brief':
          if (key.upArrow) { cycleFocusBackward(); return true; }
          if (key.downArrow) { cycleFocusForward(); return true; }
          if (key.backspace || key.delete) {
            setBrief(p => p.slice(0, -1));
          } else if (input && !key.ctrl && !key.meta) {
            setBrief(p => p + input);
          }
          break;

        case 'autoInject':
          if (key.upArrow) { cycleFocusBackward(); return true; }
          if (key.downArrow) { cycleFocusForward(); return true; }
          if (key.leftArrow || key.rightArrow) {
            setAutoInject(p => !p);
          }
          break;
      }

      return true; // Consume all input when form is active
    },
  });

  const isNameValid = name.trim().length > 0;
  const title = mode === 'create' ? 'Create Watcher Template' : 'Edit Watcher Template';
  const buttonText = mode === 'create' ? 'Create' : 'Save';

  // Show limited number of filtered models
  const maxVisibleModels = 3;
  const visibleModels = filteredModels.slice(0, maxVisibleModels);

  return (
    <Box position="absolute" flexDirection="column" width={terminalWidth} height={terminalHeight}>
      <Box flexDirection="column" flexGrow={1} backgroundColor="black">
        <Box flexDirection="column" padding={2} flexGrow={1}>
          {/* Header */}
          <Box marginBottom={1} borderStyle="single" borderBottom borderLeft={false} borderRight={false} borderTop={false}>
            <Text bold color="magenta">{title}</Text>
          </Box>

          {/* Name field */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'name' ? 'cyan' : 'white'}>
              Name{!isNameValid && focusField !== 'name' ? ' (required)' : ''}:
            </Text>
            <Box>
              <Text 
                backgroundColor={focusField === 'name' ? 'blue' : undefined} 
                color={focusField === 'name' ? 'white' : 'gray'}
              >
                {name || (focusField === 'name' ? '' : '(empty)')}{focusField === 'name' ? '▌' : ''}
              </Text>
            </Box>
          </Box>

          {/* Model field with type-to-filter */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'model' ? 'cyan' : 'white'}>Model:</Text>
            <Box flexDirection="column">
              <Box>
                <Text 
                  backgroundColor={focusField === 'model' ? 'blue' : undefined} 
                  color={focusField === 'model' ? 'white' : 'gray'}
                >
                  {focusField === 'model' ? modelFilter : selectedModel.split('/').pop()}
                  {focusField === 'model' ? '▌' : ''}
                </Text>
              </Box>
              {/* Filtered dropdown when focused */}
              {focusField === 'model' && filteredModels.length > 0 && (
                <Box flexDirection="column" marginTop={0} borderStyle="single" borderColor="gray">
                  {visibleModels.map((model, idx) => {
                    const isSelected = idx === selectedModelIndex;
                    return (
                      <Box key={model}>
                        <Text 
                          backgroundColor={isSelected ? 'magenta' : undefined}
                          color={isSelected ? 'black' : 'gray'}
                        >
                          {isSelected ? '> ' : '  '}{model}
                        </Text>
                      </Box>
                    );
                  })}
                  {filteredModels.length > maxVisibleModels && (
                    <Text dimColor>  ... and {filteredModels.length - maxVisibleModels} more</Text>
                  )}
                </Box>
              )}
              {focusField === 'model' && filteredModels.length === 0 && (
                <Text color="red" dimColor>No models match "{modelFilter}"</Text>
              )}
            </Box>
          </Box>

          {/* Authority field */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'authority' ? 'cyan' : 'white'}>Authority:</Text>
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
            </Box>
            {/* Inline explanation when focused */}
            {focusField === 'authority' && (
              <Box flexDirection="column" marginTop={0}>
                <Text dimColor italic>Peer: Suggestions the AI can consider or ignore</Text>
                <Text dimColor italic>Supervisor: Directives the AI should follow</Text>
              </Box>
            )}
          </Box>

          {/* Brief field */}
          <Box marginBottom={1} flexDirection="column" flexGrow={1}>
            <Text color={focusField === 'brief' ? 'cyan' : 'white'}>Brief (optional):</Text>
            <Box 
              borderStyle="single" 
              borderColor={focusField === 'brief' ? 'cyan' : 'gray'} 
              padding={1} 
              flexGrow={1}
            >
              <Text color={focusField === 'brief' ? 'white' : 'gray'} wrap="wrap">
                {brief || (focusField === 'brief' ? '' : '(describe what to watch for)')}{focusField === 'brief' ? '▌' : ''}
              </Text>
            </Box>
          </Box>

          {/* Auto-inject field */}
          <Box marginBottom={1} flexDirection="column">
            <Text color={focusField === 'autoInject' ? 'cyan' : 'white'}>Auto-inject:</Text>
            <Box>
              <Text 
                backgroundColor={focusField === 'autoInject' ? 'blue' : undefined} 
                color={autoInject ? 'green' : 'gray'}
              >
                {autoInject ? '[●] Enabled' : '[ ] Disabled'}
              </Text>
              {focusField === 'autoInject' && (
                <Text dimColor> (←→ to toggle)</Text>
              )}
            </Box>
          </Box>

          {/* Save button */}
          <Box marginTop={1}>
            <Text 
              backgroundColor={isNameValid ? 'green' : 'red'} 
              color="white" 
              bold
            >
              {` ${buttonText} `}
            </Text>
            {!isNameValid && (
              <Text color="red" dimColor> ← Enter a name first</Text>
            )}
          </Box>

          {/* Footer */}
          <Box marginTop={1} borderStyle="single" borderTop borderBottom={false} borderLeft={false} borderRight={false}>
            <Text dimColor>↑↓/Tab: Navigate | ←→: Toggle | Type: Filter models | Enter: Save | Esc: Cancel</Text>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
