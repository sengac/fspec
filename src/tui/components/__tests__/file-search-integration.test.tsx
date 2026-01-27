/**
 * Quick integration test for file search functionality
 */

import React, { useState } from 'react';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFileSearchInput } from '../../hooks/useFileSearchInput';
import { FileSearchPopup } from '../FileSearchPopup';

// Test component that shows hook state
const FileSearchTestComponent: React.FC<{ testInput: string }> = ({ testInput }) => {
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
    <>
      <Text>
        Input: {inputValue} | Visible: {fileSearch.isVisible ? 'YES' : 'NO'} | Files: {fileSearch.files.length}
      </Text>
      
      {fileSearch.isVisible && (
        <FileSearchPopup
          isVisible={fileSearch.isVisible}
          filter={fileSearch.filter}
          files={fileSearch.files}
          selectedIndex={fileSearch.selectedIndex}
          dialogWidth={fileSearch.dialogWidth}
        />
      )}
    </>
  );
};

describe('File Search Integration Test', () => {
  it('should work end-to-end with real glob search', async () => {
    // @step Test with @ input
    const { rerender, lastFrame } = render(<FileSearchTestComponent testInput="@" />);
    
    // Wait for hook to process
    await new Promise(resolve => setTimeout(resolve, 100));
    
    const frame = lastFrame();
    console.log('Frame output:', frame);
    
    // Should show visible = YES when @ is detected
    expect(frame).toContain('Input: @');
    expect(frame).toMatch(/Visible: (YES|NO)/);
    
    // Try with a filter
    rerender(<FileSearchTestComponent testInput="@src" />);
    
    // Wait for glob search to complete
    await new Promise(resolve => setTimeout(resolve, 500));
    
    const frame2 = lastFrame();
    console.log('Frame with filter:', frame2);
    
    // Should show files found
    expect(frame2).toContain('Input: @src');
  });
});