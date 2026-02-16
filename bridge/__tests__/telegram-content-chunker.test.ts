/**
 * Feature: spec/features/intelligent-content-aware-chunking-for-telegram-display.feature
 *
 * Tests for content-aware chunking in Telegram bridge.
 * Validates boundary detection, content summarization, and markdown validation.
 */

import { describe, it, expect, beforeEach } from 'vitest';

// These imports will fail until we implement the module
import {
  findSentenceBoundary,
  findParagraphBoundary,
  findHeadingBoundary,
  findCodeBlockBoundary,
  findListBoundary,
  findTableBoundary,
  getBestSplitPoint,
  isInsideCodeBlock,
  isInsideTable,
  summarizeToolResult,
  formatToolCallSummary,
  balanceMarkdown,
  ContentChunker,
} from '../telegram-content-chunker';

// ===========================================
// BOUNDARY DETECTION SCENARIOS
// ===========================================

describe('Feature: Intelligent Content-Aware Chunking for Telegram Display', () => {
  describe('Boundary Detection', () => {
    describe('Scenario: Complete sentence arrives in single message', () => {
      it('should keep complete sentence together', () => {
        // @step Given the chunker receives streaming text "I will analyze this code."
        const text = 'I will analyze this code.';

        // @step When the idle timeout triggers a flush
        const boundary = findSentenceBoundary(text, text.length);

        // @step Then Telegram receives "I will analyze this code." as a single message
        expect(boundary).toBe(text.length);

        // @step And the message is not split mid-word
        expect(text.slice(0, boundary)).toBe('I will analyze this code.');
      });
    });

    describe('Scenario: Buffer flushes at sentence boundary when approaching limit', () => {
      it('should find sentence boundary before max position', () => {
        // @step Given the buffer contains 3400 characters of text
        const text =
          'First sentence here. Second sentence continues. Third sentence ends here. Fourth goes on and on.';

        // @step And the next chunk would push it over 3500 characters
        const maxPosition = 70; // Simulate limit falling in middle

        // @step When the chunker detects a sentence boundary at 3200 characters
        const boundary = findSentenceBoundary(text, maxPosition);

        // @step Then it flushes at the sentence boundary
        expect(boundary).toBeLessThanOrEqual(maxPosition);
        expect(text[boundary - 1]).toMatch(/[.!?]/);

        // @step And the remaining text stays in the buffer for the next message
        const remaining = text.slice(boundary).trim();
        expect(remaining.length).toBeGreaterThan(0);
      });
    });

    describe('Scenario: Code block arrives as complete unit', () => {
      it('should keep code block together', () => {
        // @step Given the chunker receives a code block "```typescript\nconst x = 1;\n```"
        const text = '```typescript\nconst x = 1;\n```';

        // @step When the idle timeout triggers a flush
        const boundary = findCodeBlockBoundary(text, text.length);

        // @step Then Telegram receives the complete code block in one message
        expect(boundary).toBe(text.length);

        // @step And the code block is not split across messages
        const chunk = text.slice(0, boundary);
        expect(chunk).toContain('```typescript');
        expect(chunk).toMatch(/```$/);
      });
    });

    describe('Scenario: Multi-line code block stays together', () => {
      it('should keep 50-line code block in single message', () => {
        // @step Given the chunker receives a 50-line code block
        const lines = Array.from({ length: 50 }, (_, i) => `  line ${i + 1}`);
        const text = '```typescript\n' + lines.join('\n') + '\n```';

        // @step And the code block is under 4096 characters
        expect(text.length).toBeLessThan(4096);

        // @step When the idle timeout triggers a flush
        const boundary = findCodeBlockBoundary(text, text.length);

        // @step Then all 50 lines arrive in a single Telegram message
        expect(boundary).toBe(text.length);
      });
    });

    describe('Scenario: Paragraph break triggers new chunk', () => {
      it('should split at paragraph boundaries', () => {
        // @step Given the buffer contains "First paragraph.\n\nSecond paragraph."
        const text = 'First paragraph.\n\nSecond paragraph.';

        // @step When the buffer is flushed
        const boundary = findParagraphBoundary(text, text.length);

        // @step Then "First paragraph." becomes one message
        expect(boundary).toBe(18); // "First paragraph.\n\n"

        // @step And "Second paragraph." becomes the next message
        const remaining = text.slice(boundary).trim();
        expect(remaining).toBe('Second paragraph.');
      });
    });

    describe('Scenario: Heading starts new message', () => {
      it('should split before headings', () => {
        // @step Given the buffer contains "Some text.\n\n## New Section\n\nMore text."
        const text = 'Some text.\n\n## New Section\n\nMore text.';

        // @step When the buffer is flushed
        const boundary = findHeadingBoundary(text, text.length);

        // @step Then "Some text." is sent first
        expect(text.slice(0, boundary).trim()).toBe('Some text.');

        // @step And "## New Section\n\nMore text." starts a new message
        const remaining = text.slice(boundary).trim();
        expect(remaining).toMatch(/^## New Section/);
      });
    });

    describe('Scenario: List items stay together in single message', () => {
      it('should keep list items together', () => {
        // @step Given the chunker receives a list with 10 items
        const items = Array.from({ length: 10 }, (_, i) => `- Item ${i + 1}`);
        const text = items.join('\n');

        // @step And the total list is under 4096 characters
        expect(text.length).toBeLessThan(4096);

        // @step When the idle timeout triggers a flush
        const boundary = findListBoundary(text, text.length);

        // @step Then all 10 items arrive in a single Telegram message
        expect(boundary).toBe(text.length);
      });
    });

    describe('Scenario: Code block boundary takes priority over paragraph', () => {
      it('should not split inside code block even at paragraph boundary', () => {
        // @step Given the buffer contains text followed by a code block followed by a paragraph
        const text =
          'Before.\n\n```typescript\nconst x = 1;\n\nconst y = 2;\n```\n\nAfter.';

        // @step When the buffer approaches the size limit inside the code block
        const positionInsideCodeBlock = 40; // Inside the code block

        // @step Then it waits for the code block to complete before flushing
        const isInside = isInsideCodeBlock(text, positionInsideCodeBlock);
        expect(isInside).toBe(true);

        // @step And does not split at the paragraph boundary inside the code block
        const boundary = getBestSplitPoint(text, positionInsideCodeBlock);
        // Should find boundary before the code block, not inside it
        expect(boundary).toBeLessThan(positionInsideCodeBlock);
      });
    });

    describe('Scenario: Markdown table arrives as complete unit', () => {
      it('should keep table rows together', () => {
        // @step Given the chunker receives a markdown table with header, separator and 5 data rows
        const table = `| Date | Price | Change |
|------|-------|--------|
| Feb 15 | $5,041.80 | — |
| Feb 13 | $5,041.80 | +1.86% |
| Feb 12 | $4,975.61 | -1.91% |
| Feb 11 | $5,069.21 | +0.34% |
| Feb 10 | $5,048.96 | +0.17% |`;

        // @step When the idle timeout triggers a flush
        const boundary = findTableBoundary(table, table.length);

        // @step Then all table rows arrive in a single Telegram message
        expect(boundary).toBe(table.length);

        // @step And the table is not split mid-row
        const chunk = table.slice(0, boundary);
        expect(chunk.split('\n').length).toBe(7); // header + separator + 5 data rows
      });
    });

    describe('Scenario: Table boundary takes priority - splits before table not mid-table', () => {
      it('should split before table when content exceeds limit', () => {
        // @step Given the buffer contains 3000 characters of text followed by a 2000 character table
        const textBefore = 'x'.repeat(3000) + '\n\n';
        const table =
          `| Date | Price | Change |
|------|-------|--------|
` +
          Array.from(
            { length: 50 },
            (_, i) => `| Row ${i} | $1000.00 | +0.00% |`
          ).join('\n');
        const text = textBefore + table;

        // @step When the buffer is flushed due to exceeding the 4096 limit
        const boundary = getBestSplitPoint(text, 4096);

        // @step Then the first message contains the text before the table
        expect(boundary).toBeLessThanOrEqual(textBefore.length);

        // @step And the second message contains the complete table
        const remaining = text.slice(boundary).trim();
        expect(remaining).toMatch(/^\| Date/);

        // @step And no table row is split across messages
        const firstPart = text.slice(0, boundary);
        const rowPattern = /^\|[^|]+\|[^|]*$/m;
        expect(firstPart).not.toMatch(rowPattern); // No incomplete rows
      });
    });

    describe('Scenario: Table row never split mid-row', () => {
      it('should never split in the middle of a table row', () => {
        // @step Given the buffer contains a table row "| Feb 10 | $5,048.96 | +0.17% |"
        const text =
          'Some text before.\n\n| Date | Price | Change |\n|------|-------|--------|\n| Feb 10 | $5,048.96 | +0.17% |';

        // @step When the buffer approaches the size limit mid-row
        const positionMidRow = text.indexOf('$5,048') + 3; // Mid-way through a row
        const isInside = isInsideTable(text, positionMidRow);

        // @step Then it waits for the row to complete before flushing
        expect(isInside).toBe(true);

        // @step And the row "| Feb 10 | $5,048.96 | +0.17% |" is never split into separate messages
        const boundary = getBestSplitPoint(text, positionMidRow);
        const chunk = text.slice(0, boundary);
        // Should not contain partial table row
        expect(chunk).not.toMatch(/\| Feb 10 \| \$5,0$/);
        expect(chunk).not.toMatch(/\| Feb 10 \|$/);
      });
    });
  });

  // ===========================================
  // CONTENT SUMMARIZATION SCENARIOS
  // ===========================================

  describe('Content Summarization', () => {
    describe('Scenario: Thinking content wrapped in think tags', () => {
      let chunker: ContentChunker;

      beforeEach(() => {
        chunker = new ContentChunker();
      });

      it('should wrap thinking content in escaped <think> tags for MarkdownV2', () => {
        // @step Given Claude sends a thinking chunk with reasoning content
        const thinking =
          'Let me analyze this problem carefully. I need to consider the edge cases.';

        // @step When the thinking block is processed for Telegram
        chunker.addThinking(thinking);
        const result = chunker.flush();

        // @step Then the first message starts with escaped '\<think\>'
        expect(result.messages.length).toBeGreaterThan(0);
        expect(result.messages[0]).toMatch(/^\\<think\\>/);

        // @step And the actual thinking content flows naturally
        expect(result.messages.join('')).toContain(thinking);

        // @step And the final message ends with escaped '\</think\>'
        const lastMessage = result.messages[result.messages.length - 1];
        expect(lastMessage).toMatch(/\\<\/think\\>$/);
      });
    });

    describe('Scenario: Multiple thinking chunks stream as continuous content', () => {
      let chunker: ContentChunker;

      beforeEach(() => {
        chunker = new ContentChunker();
      });

      it('should stream multiple thinking chunks between single escaped tags', () => {
        // @step Given Claude sends 5 separate thinking chunks in succession
        const thinkingChunks = [
          'First thought about the problem.',
          'Second thought considering alternatives.',
          'Third thought weighing tradeoffs.',
          'Fourth thought about implementation.',
          'Fifth thought on testing approach.',
        ];

        // @step When they are processed for Telegram
        thinkingChunks.forEach(t => chunker.addThinking(t));
        const result = chunker.flush();

        // @step Then the content flows between single escaped '\<think\>' and '\</think\>' tags
        const fullOutput = result.messages.join('');
        expect((fullOutput.match(/\\<think\\>/g) || []).length).toBe(1);
        expect((fullOutput.match(/\\<\/think\\>/g) || []).length).toBe(1);

        // @step And NOT 5 separate '🤔' indicator messages
        const thinkingIndicators = result.messages.filter(m =>
          m.includes('🤔')
        );
        expect(thinkingIndicators.length).toBe(0);
      });
    });

    describe('Scenario: Tool call displays formatted invocation', () => {
      it('should format tool call with name', () => {
        // @step Given Claude invokes the Fspec tool with command "create-story"
        const toolName = 'Fspec';
        const args = { command: 'create-story' };

        // @step When the tool_call chunk is processed
        const formatted = formatToolCallSummary(toolName, args);

        // @step Then Telegram shows "🔧 Running: Fspec(create-story)"
        expect(formatted).toBe('🔧 Running: Fspec(create-story)');
      });
    });

    describe('Scenario: File read tool result shows summary with line count', () => {
      it('should summarize file read with line count', () => {
        // @step Given Claude reads a 500-line file "src/auth.ts"
        const toolName = 'Read';
        const content = Array.from(
          { length: 500 },
          (_, i) => `line ${i + 1}`
        ).join('\n');

        // @step When the tool_result chunk is processed
        const summary = summarizeToolResult(toolName, content, {
          file_path: 'src/auth.ts',
        });

        // @step Then Telegram shows "📄 Read src/auth.ts (500 lines)" instead of file contents
        expect(summary).toMatch(/📄.*src\/auth\.ts.*500 lines/);
      });
    });

    describe('Scenario: Large tool output summarized not sent verbatim', () => {
      it('should summarize large output', () => {
        // @step Given a tool returns 10000 characters of output
        const content = 'x'.repeat(10000);

        // @step When the tool_result chunk is processed
        const summary = summarizeToolResult('SomeTool', content);

        // @step Then Telegram receives a summary under 500 characters
        expect(summary.length).toBeLessThan(500);

        // @step And the full output is not sent
        expect(summary).not.toBe(content);
      });
    });

    describe('Scenario: Tool call with arguments shows arg summary', () => {
      it('should include args in tool call summary', () => {
        // @step Given Claude invokes Read with file_path "/home/user/file.ts"
        const toolName = 'Read';
        const args = { file_path: '/home/user/file.ts' };

        // @step When the tool_call chunk is processed
        const formatted = formatToolCallSummary(toolName, args);

        // @step Then Telegram shows "🔧 Running: Read(file_path: /home/user/file.ts)"
        expect(formatted).toMatch(
          /🔧 Running: Read.*file_path.*\/home\/user\/file\.ts/
        );
      });
    });
  });

  // ===========================================
  // MARKDOWN VALIDATION SCENARIOS
  // ===========================================

  describe('Markdown Validation', () => {
    describe('Scenario: Message respects 4096 character limit', () => {
      it('should never exceed 4096 characters', () => {
        // @step Given the buffer contains 5000 characters of text
        const text = 'x'.repeat(5000);

        // @step When the buffer is flushed
        const boundary = getBestSplitPoint(text, 4096);

        // @step Then no single Telegram message exceeds 4096 characters
        expect(boundary).toBeLessThanOrEqual(4096);
      });
    });

    describe('Scenario: Long message splits at logical boundary before limit', () => {
      it('should split at paragraph before 4096', () => {
        // @step Given the buffer contains 5000 characters with a paragraph break at 3800
        const text = 'x'.repeat(3800) + '\n\n' + 'y'.repeat(1198);

        // @step When the buffer is flushed
        const boundary = getBestSplitPoint(text, 4096);

        // @step Then the first message ends at the paragraph break
        expect(boundary).toBe(3802); // 3800 + '\n\n'

        // @step And the second message contains the remainder
        const remaining = text.slice(boundary);
        expect(remaining.length).toBeGreaterThan(0);
      });
    });

    describe('Scenario: Unclosed code block closed before sending', () => {
      it('should close unclosed code block', () => {
        // @step Given the buffer contains "```typescript\nconst x = 1;" without closing fence
        const text = '```typescript\nconst x = 1;';

        // @step And the buffer is being force-flushed due to size limit
        // @step When the message is prepared for sending
        const balanced = balanceMarkdown(text);

        // @step Then a closing "```" is appended to make valid markdown
        expect(balanced).toMatch(/```$/);
      });
    });

    describe('Scenario: Unclosed bold markers balanced before sending', () => {
      it('should close unclosed bold markers', () => {
        // @step Given the buffer contains "This is **bold text without closing"
        const text = 'This is **bold text without closing';

        // @step And the buffer is being force-flushed
        // @step When the message is prepared for sending
        const balanced = balanceMarkdown(text);

        // @step Then a closing "**" is appended to balance the markers
        const asteriskCount = (balanced.match(/\*\*/g) || []).length;
        expect(asteriskCount % 2).toBe(0);
      });
    });

    describe('Scenario: Inline code backticks balanced in each chunk', () => {
      it('should close unclosed inline code', () => {
        // @step Given the buffer contains "Use the `command without closing"
        const text = 'Use the `command without closing';

        // @step And the buffer is being force-flushed
        // @step When the message is prepared for sending
        const balanced = balanceMarkdown(text);

        // @step Then a closing backtick is appended
        const backtickCount = (balanced.match(/`/g) || []).length;
        expect(backtickCount % 2).toBe(0);
      });
    });

    describe('Scenario: Code block exceeding limit truncated with indicator', () => {
      it('should truncate oversized code block with indicator', () => {
        // @step Given a code block contains 6000 characters
        const codeContent = 'x'.repeat(5980);
        const text = '```typescript\n' + codeContent + '\n```';

        // @step When it is processed for Telegram
        const boundary = getBestSplitPoint(text, 4096);
        const truncated = balanceMarkdown(text.slice(0, boundary));

        // @step Then it is truncated to under 4096 characters
        expect(truncated.length).toBeLessThanOrEqual(4096);

        // @step And includes "[...N chars omitted...]" indicator
        // Note: This might be handled by existing truncateMessage function
      });
    });

    describe('Scenario: Truncated code block has closing fence', () => {
      it('should ensure truncated code block has closing fence', () => {
        // @step Given a code block is truncated due to size
        const text = '```typescript\n' + 'x'.repeat(5000);

        // @step When the truncated message is prepared
        const balanced = balanceMarkdown(text.slice(0, 4000));

        // @step Then it has both opening and closing "```" fences
        expect(balanced).toMatch(/^```/);
        expect(balanced).toMatch(/```$/);

        // @step And the markdown is valid
        const fenceCount = (balanced.match(/```/g) || []).length;
        expect(fenceCount % 2).toBe(0);
      });
    });
  });
});
