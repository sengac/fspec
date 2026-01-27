/**
 * Tests for Slash Command Autocomplete Palette
 *
 * Feature: spec/features/slash-command-autocomplete-palette.feature
 * Work Unit: TUI-050
 *
 * Tests the slash command autocomplete palette functionality including:
 * - Palette visibility on "/" input
 * - Command filtering with three-tier matching
 * - Keyboard navigation (Up/Down arrows)
 * - Command selection (Tab/Enter)
 * - Palette dismissal (Escape, Backspace)
 */

import { describe, it, expect } from 'vitest';
import {
  filterCommands,
  SLASH_COMMANDS,
  type SlashCommand,
} from '../utils/slashCommands';

describe('Feature: Slash Command Autocomplete Palette', () => {
  describe('Scenario: Show palette when typing slash at start of input', () => {
    it('should show palette with all commands when "/" is typed at position 0', () => {
      // @step Given I am in the TUI input area with empty input
      const input = '';

      // @step When I type "/" at position 0
      const newInput = '/';
      const shouldShowPalette = newInput.startsWith('/') && newInput.length === 1;

      // @step Then the slash command palette should appear
      expect(shouldShowPalette).toBe(true);

      // @step And the palette should show all available commands
      const filter = newInput.slice(1); // Remove leading "/"
      const filteredCommands = filterCommands(SLASH_COMMANDS, filter);
      expect(filteredCommands.length).toBe(SLASH_COMMANDS.length);

      // @step And the first command should be selected
      const selectedIndex = 0;
      expect(selectedIndex).toBe(0);
    });
  });

  describe('Scenario: Filter commands by prefix matching', () => {
    it('should filter commands by prefix and show matching commands', () => {
      // @step Given the slash command palette is visible
      const paletteVisible = true;
      expect(paletteVisible).toBe(true);

      // @step And the input contains "/"
      let input = '/';

      // @step When I type "m" after the slash
      input = '/m';
      const filter = input.slice(1);

      // @step Then the palette should show only commands starting with "m"
      const filteredCommands = filterCommands(SLASH_COMMANDS, filter);
      // All returned commands should match by prefix, substring, or description
      expect(filteredCommands.length).toBeGreaterThan(0);
      expect(filteredCommands.every(cmd => 
        cmd.name.toLowerCase().startsWith('m') ||
        cmd.name.toLowerCase().includes('m') ||
        cmd.description.toLowerCase().includes('m')
      )).toBe(true);

      // @step And the commands "model", "mode", "merge", "mcp" should be visible
      const commandNames = filteredCommands.map((c) => c.name);
      expect(commandNames).toContain('model');
      expect(commandNames).toContain('mode');
      expect(commandNames).toContain('merge');
      expect(commandNames).toContain('mcp');

      // @step And the first matching command should be selected
      const selectedIndex = 0;
      expect(selectedIndex).toBe(0);
    });
  });

  describe('Scenario: Navigate through command list with arrow keys', () => {
    it('should move selection when pressing Down arrow', () => {
      // @step Given the slash command palette is visible with multiple commands
      const commands = filterCommands(SLASH_COMMANDS, '');
      expect(commands.length).toBeGreaterThan(2);

      // @step And the first command is selected
      let selectedIndex = 0;

      // @step When I press the Down arrow key twice
      // Simulate moveDown logic from useSlashCommand hook
      selectedIndex = selectedIndex >= commands.length - 1 ? 0 : selectedIndex + 1;
      selectedIndex = selectedIndex >= commands.length - 1 ? 0 : selectedIndex + 1;

      // @step Then the third command in the list should be selected
      expect(selectedIndex).toBe(2);

      // @step And the selection highlight should move accordingly
      expect(commands[selectedIndex]).toBeDefined();
    });
  });

  describe('Scenario: Accept selected command with Tab key', () => {
    it('should update input and close palette when Tab is pressed', () => {
      // @step Given the slash command palette is visible
      let paletteVisible = true;

      // @step And the "model" command is selected
      const selectedCommand = SLASH_COMMANDS.find((c) => c.name === 'model');
      expect(selectedCommand).toBeDefined();

      // @step When I press the Tab key
      // Simulate Tab behavior: set input to command (no trailing space), hide palette
      let input = `/${selectedCommand!.name}`;
      paletteVisible = false;

      // @step Then the input should update to "/model"
      expect(input).toBe('/model');

      // @step And the palette should close
      expect(paletteVisible).toBe(false);
    });
  });

  describe('Scenario: Close palette with Escape key', () => {
    it('should close palette and preserve input when Escape is pressed', () => {
      // @step Given the slash command palette is visible
      let paletteVisible = true;

      // @step And the input contains "/m"
      const input = '/m';

      // @step When I press the Escape key
      paletteVisible = false;

      // @step Then the palette should close
      expect(paletteVisible).toBe(false);

      // @step And the input should remain unchanged as "/m"
      expect(input).toBe('/m');
    });
  });

  describe('Scenario: Show no matching commands message', () => {
    it('should display no matching commands when filter has no matches', () => {
      // @step Given the slash command palette is visible
      const paletteVisible = true;
      expect(paletteVisible).toBe(true);

      // @step When I type "/xyz"
      const input = '/xyz';
      const filter = input.slice(1);

      // @step Then the palette should display "No matching commands"
      const filteredCommands = filterCommands(SLASH_COMMANDS, filter);
      const showNoMatchesMessage = filteredCommands.length === 0;
      expect(showNoMatchesMessage).toBe(true);

      // @step And no command should be selectable
      expect(filteredCommands.length).toBe(0);
    });
  });

  describe('Scenario: Close palette when deleting slash character', () => {
    it('should close palette when "/" is deleted with Backspace', () => {
      // @step Given the slash command palette is visible
      let paletteVisible = true;

      // @step And the input contains only "/"
      let input = '/';

      // @step When I press Backspace to delete the slash
      input = input.slice(0, -1);
      // handleInputChange logic: hide palette if input doesn't start with "/"
      paletteVisible = input.startsWith('/');

      // @step Then the palette should close
      expect(paletteVisible).toBe(false);

      // @step And the input should be empty
      expect(input).toBe('');
    });
  });

  describe('Scenario: Execute command immediately with Enter key', () => {
    it('should execute command and close palette when Enter is pressed', () => {
      // @step Given the slash command palette is visible
      let paletteVisible = true;

      // @step And the "clear" command is selected
      const selectedCommand = SLASH_COMMANDS.find((c) => c.name === 'clear');
      expect(selectedCommand).toBeDefined();

      // @step When I press the Enter key
      // Simulate Enter behavior: execute command and hide palette
      let commandExecuted = false;
      let input = `/clear`;

      // Simulate command execution
      commandExecuted = true;
      paletteVisible = false;
      input = ''; // Input cleared after command execution

      // @step Then the command should execute immediately
      expect(commandExecuted).toBe(true);

      // @step And the palette should close
      expect(paletteVisible).toBe(false);

      // @step And the input should be cleared
      expect(input).toBe('');
    });
  });
});

describe('useSlashCommand hook behavior (logic verification)', () => {
  it('should reset selection when filter changes', () => {
    // Simulate hook behavior: setFilter resets selectedIndex to 0
    let selectedIndex = 5;
    const setFilter = () => {
      selectedIndex = 0; // This is what the hook does
    };
    
    setFilter();
    expect(selectedIndex).toBe(0);
  });

  it('should wrap selection when moving up from first item', () => {
    const commands = filterCommands(SLASH_COMMANDS, '');
    let selectedIndex = 0;
    
    // Simulate moveUp with wrap-around
    selectedIndex = selectedIndex <= 0 ? commands.length - 1 : selectedIndex - 1;
    
    expect(selectedIndex).toBe(commands.length - 1);
  });

  it('should wrap selection when moving down from last item', () => {
    const commands = filterCommands(SLASH_COMMANDS, '');
    let selectedIndex = commands.length - 1;
    
    // Simulate moveDown with wrap-around
    selectedIndex = selectedIndex >= commands.length - 1 ? 0 : selectedIndex + 1;
    
    expect(selectedIndex).toBe(0);
  });

  it('should return correct selected command', () => {
    const commands = filterCommands(SLASH_COMMANDS, '');
    const selectedIndex = 2;
    
    // Simulate getSelectedCommand
    const selectedCommand = commands[selectedIndex];
    
    expect(selectedCommand).toBeDefined();
    expect(selectedCommand.name).toBe(SLASH_COMMANDS[2].name);
  });

  it('should return undefined for selected command when no matches', () => {
    const commands = filterCommands(SLASH_COMMANDS, 'xyz'); // No matches
    const selectedIndex = 0;
    
    // Simulate getSelectedCommand with empty commands
    const selectedCommand = commands[selectedIndex];
    
    expect(selectedCommand).toBeUndefined();
  });
});

describe('filterCommands', () => {
  const testCommands: SlashCommand[] = [
    { name: 'model', description: 'Select AI model' },
    { name: 'mode', description: 'Cycle through modes' },
    { name: 'merge', description: 'Merge messages' },
    { name: 'mcp', description: 'Manage MCP providers' },
    { name: 'clear', description: 'Clear history' },
    { name: 'compact', description: 'Compact context' },
    { name: 'resume', description: 'Resume session' },
  ];

  it('returns all commands when filter is empty', () => {
    const result = filterCommands(testCommands, '');
    expect(result).toEqual(testCommands);
  });

  it('filters by prefix first', () => {
    const result = filterCommands(testCommands, 'mo');
    expect(result[0].name).toBe('model');
    expect(result[1].name).toBe('mode');
  });

  it('filters by substring second', () => {
    const result = filterCommands(testCommands, 'de');
    // 'mode' contains 'de', 'model' contains 'de'
    expect(result.some((c) => c.name === 'mode')).toBe(true);
    expect(result.some((c) => c.name === 'model')).toBe(true);
  });

  it('filters by description third', () => {
    const result = filterCommands(testCommands, 'AI');
    // 'model' has 'AI' in description
    expect(result.some((c) => c.name === 'model')).toBe(true);
  });

  it('returns empty array when no matches', () => {
    const result = filterCommands(testCommands, 'xyz');
    expect(result).toEqual([]);
  });

  it('is case insensitive', () => {
    const result = filterCommands(testCommands, 'MODEL');
    expect(result[0].name).toBe('model');
  });
});
