import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import type { AgentConfig } from '../utils/agentRegistry';

interface AgentSelectorProps {
  agents: AgentConfig[];
  preSelected: string[];
  onSubmit: (selected: string) => void;
}

export const AgentSelector: React.FC<AgentSelectorProps> = ({
  agents,
  preSelected,
  onSubmit,
}) => {
  // Start cursor on first pre-selected agent, or first agent if none detected
  const initialCursor = preSelected.length > 0
    ? agents.findIndex(a => a.id === preSelected[0])
    : 0;
  const [cursor, setCursor] = useState(initialCursor >= 0 ? initialCursor : 0);

  useInput((input, key) => {
    if (key.upArrow) {
      setCursor(Math.max(0, cursor - 1));
    } else if (key.downArrow) {
      setCursor(Math.min(agents.length - 1, cursor + 1));
    } else if (key.return) {
      // Select the agent at current cursor position
      onSubmit(agents[cursor].id);
    }
  });

  return (
    <Box flexDirection="column">
      <Text bold>Select your AI coding agent:</Text>
      <Text dimColor>(Use ↑↓ to navigate, ENTER to select)</Text>
      <Text> </Text>
      {agents.map((agent, index) => {
        const isCursor = index === cursor;
        const marker = isCursor ? '▶' : ' ';

        return (
          <Text key={agent.id} color={isCursor ? 'cyan' : undefined}>
            {marker} {agent.name}
            {preSelected.includes(agent.id) && (
              <Text dimColor> (detected)</Text>
            )}
          </Text>
        );
      })}
    </Box>
  );
};
