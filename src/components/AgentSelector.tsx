import React, { useState, useEffect } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import type { AgentConfig } from '../utils/agentRegistry';
import { getActivationMessage } from '../utils/activationMessage';

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
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const { exit } = useApp();

  useInput((input, key) => {
    if (selectedAgent) {
      return;
    }

    if (key.upArrow) {
      setCursor(Math.max(0, cursor - 1));
    } else if (key.downArrow) {
      setCursor(Math.min(agents.length - 1, cursor + 1));
    } else if (key.return) {
      // Select the agent
      setSelectedAgent(agents[cursor].id);
    }
  });

  useEffect(() => {
    if (selectedAgent) {
      onSubmit(selectedAgent);
      // Exit after a brief delay to show the success message
      setTimeout(() => {
        exit();
      }, 100);
    }
  }, [selectedAgent, onSubmit, exit]);

  if (selectedAgent) {
    const agent = agents.find(a => a.id === selectedAgent);
    const agentName = agent?.name || selectedAgent;
    return (
      <Box flexDirection="column">
        <Text color="green">✓ Installed fspec for {agentName}</Text>
      </Box>
    );
  }

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
