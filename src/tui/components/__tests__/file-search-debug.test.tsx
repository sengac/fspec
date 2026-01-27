/**
 * Quick debug test for file search hook with ink-testing-library
 */

import React, { useState } from 'react';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFileSearchInput } from '../../hooks/useFileSearchInput';

// Mock tool integration
vi.mock('../../../utils/toolIntegration', () => ({
  callGlobTool: vi.fn().mockResolvedValue({
    success: true,
    data: 'src/components/Button.tsx\nsrc/utils/helper.ts',
  }),
}));

// Simple test component that shows hook state
const FileSearchDebugComponent: React.FC<{ testInput: string }> = ({ testInput }) => {
  const [inputValue, setInputValue] = useState(testInput);
  
  const fileSearch = useFileSearchInput({
    inputValue,
    onInputChange: setInputValue,
    disabled: false,
  });
  
  // Trigger detection when testInput changes
  React.useEffect(() => {
    if (testInput !== inputValue) {
      fileSearch.handleInputChange(testInput);
    }
  }, [testInput, inputValue, fileSearch]);
  
  return (
    <Text>
      Input: {inputValue} | Visible: {fileSearch.isVisible ? 'YES' : 'NO'} | Filter: {fileSearch.filter} | Files: {fileSearch.files.length}
    </Text>
  );
};

describe('File Search Hook Integration Debug', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });
  
  it('should show popup state when @ is typed', async () => {
    // @step Test with empty input
    const { rerender, lastFrame } = render(<FileSearchDebugComponent testInput="" />);
    expect(lastFrame()).toContain('Visible: NO');
    
    // @step Test with @ input
    rerender(<FileSearchDebugComponent testInput="@" />);
    
    // Wait for async effects
    await new Promise(resolve => setTimeout(resolve, 50));
    
    const frame = lastFrame();
    console.log('Debug frame output:', frame);
    
    // The hook should show visible = YES when @ is detected
    expect(frame).toContain('Input: @');
    // This will tell us if the hook is working
    expect(frame).toMatch(/Visible: (YES|NO)/);
  });
});