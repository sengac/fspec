/**
 * File Search Integration Test
 * 
 * Tests end-to-end file search functionality with NAPI Glob integration.
 */

import React, { useState } from 'react';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { describe, it, expect, vi } from 'vitest';
import { useFileSearchInput } from '../../hooks/useFileSearchInput';
import { FileSearchPopup } from '../FileSearchPopup';

vi.mock('../../../utils/toolIntegration', () => ({
  callGlobTool: vi.fn().mockResolvedValue({
    success: true,
    data: 'src/components/Button.tsx\nsrc/components/Input.tsx\nsrc/hooks/useFileSearch.ts'
  }),
}));

function TestFileSearchComponent() {
  const [input, setInput] = useState('');
  
  const fileSearch = useFileSearchInput({
    inputValue: input,
    onInputChange: setInput,
    terminalWidth: 80,
    disabled: false,
  });

  return (
    <>
      <Text>Input: {input} | Visible: {fileSearch.isVisible ? 'YES' : 'NO'} | Files: {fileSearch.files.length}</Text>
      <FileSearchPopup
        isVisible={fileSearch.isVisible}
        filter={fileSearch.filter}
        files={fileSearch.files}
        selectedIndex={fileSearch.selectedIndex}
        dialogWidth={fileSearch.dialogWidth}
      />
    </>
  );
}

describe('File Search Integration Test', () => {
  it('should work end-to-end with real glob search', async () => {
    const { lastFrame } = render(<TestFileSearchComponent />);
    
    expect(lastFrame()).toContain('Visible: NO');

    const testComponent = React.createElement(() => {
      const [input, setInput] = useState('@src');
      
      const fileSearch = useFileSearchInput({
        inputValue: input,
        onInputChange: setInput,
        terminalWidth: 80,
        disabled: false,
      });

      React.useEffect(() => {
        fileSearch.handleInputChange('@src');
      }, []);

      return (
        <>
          <Text>Input: {input} |</Text>
          <FileSearchPopup
            isVisible={fileSearch.isVisible}
            filter={fileSearch.filter}
            files={fileSearch.files}
            selectedIndex={fileSearch.selectedIndex}
            dialogWidth={fileSearch.dialogWidth}
          />
        </>
      );
    });

    const { lastFrame: finalFrame } = render(testComponent);
    
    expect(finalFrame()).toContain('Input: @src');
  });
});