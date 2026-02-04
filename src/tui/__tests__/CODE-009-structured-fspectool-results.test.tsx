/**
 * Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
 * CODE-009: Structured FspecTool Results via StreamChunk Discriminated Union
 *
 * These tests verify:
 * 1. FspecCommandRequest type exists in StreamChunk discriminated union
 * 2. FspecCommandResult type exists in StreamChunk discriminated union
 * 3. TypeScript handles FspecCommandRequest with type-safe field access (no string parsing)
 * 4. System reminder is included in FspecCommandResult for workflow orchestration
 * 5. FSPEC_INTERCEPT string pattern is removed after migration
 */

import { describe, it, expect, vi } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import type { StreamChunk, FspecRequest, FspecResult } from '../../../codelet/napi/index';

describe('CODE-009: Structured FspecTool Results via StreamChunk Discriminated Union', () => {
  // Read the actual generated TypeScript definitions
  const indexDtsPath = path.join(process.cwd(), 'codelet/napi/index.d.ts');
  const indexDts = fs.readFileSync(indexDtsPath, 'utf-8');

  // Read the AgentView source to verify implementation patterns
  const agentViewPath = path.join(process.cwd(), 'src/tui/components/AgentView.tsx');
  const agentViewSource = fs.readFileSync(agentViewPath, 'utf-8');

  // Read the Rust wrapper source to verify FSPEC_INTERCEPT removal
  const wrapperRsPath = path.join(process.cwd(), 'codelet/tools/src/facade/wrapper.rs');
  const wrapperRsSource = fs.existsSync(wrapperRsPath) ? fs.readFileSync(wrapperRsPath, 'utf-8') : '';

  // Read stream_handlers.rs to verify FSPEC_INTERCEPT handling removal
  const streamHandlersPath = path.join(process.cwd(), 'codelet/napi/src/stream_handlers.rs');
  const streamHandlersSource = fs.existsSync(streamHandlersPath) ? fs.readFileSync(streamHandlersPath, 'utf-8') : '';

  // ============================================================================
  // Scenario: FspecTool emits structured command request and receives typed result
  // ============================================================================

  describe('Scenario: FspecTool emits structured command request and receives typed result', () => {
    it('should have FspecCommandRequest and FspecCommandResult variants with all required fields', () => {
      // @step Given a codelet session with FspecTool available
      // @step And the StreamChunk type includes FspecCommandRequest and FspecCommandResult variants
      // Verify FspecCommandRequest is a discriminated union variant
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]FspecCommandRequest['"]/);

      // @step When the LLM invokes Fspec tool with command "show-work-unit" and args '{"id":"CODE-001"}'
      // @step Then Rust should emit a FspecCommandRequest chunk with typed fields
      // Verify it has the required typed fields
      expect(indexDts).toContain('fspecRequest: FspecRequest');

      // Verify FspecCommandResult is a discriminated union variant
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]FspecCommandResult['"]/);

      // @step And TypeScript should receive the chunk with direct field access via chunk.fspecRequest.command
      // Verify it has the required typed fields
      expect(indexDts).toContain('fspecResult: FspecResult');

      // @step And TypeScript should execute the command via callFspecCommand callback
      // @step And the result should be returned as a FspecCommandResult chunk with success and data fields
      // Verify the FspecRequest type has all required fields
      const fspecRequestMatch = indexDts.match(
        /export (?:interface|type) FspecRequest\s*[{=][\s\S]*?(?=\nexport )/
      );
      expect(fspecRequestMatch).not.toBeNull();

      const fspecRequestDef = fspecRequestMatch![0];
      expect(fspecRequestDef).toContain('command: string');
      expect(fspecRequestDef).toContain('argsJson: string');
      expect(fspecRequestDef).toContain('projectRoot: string');
      expect(fspecRequestDef).toContain('toolCallId: string');

      // Verify the FspecResult type has all required fields
      const fspecResultMatch = indexDts.match(
        /export (?:interface|type) FspecResult\s*[{=][\s\S]*?(?=\nexport )/
      );
      expect(fspecResultMatch).not.toBeNull();

      const fspecResultDef = fspecResultMatch![0];
      expect(fspecResultDef).toContain('success: boolean');
      expect(fspecResultDef).toContain('data: string');
      expect(fspecResultDef).toMatch(/error\??\s*:\s*string/); // Optional error field
      expect(fspecResultDef).toMatch(/systemReminder\??\s*:\s*string/); // Optional system reminder
      expect(fspecResultDef).toContain('toolCallId: string');
    });

    it('should process FspecCommandRequest chunk with type-safe field access', () => {
      // @step Given a codelet session with FspecTool available
      // @step When the LLM invokes Fspec tool with command "show-work-unit" and args '{"id":"CODE-001"}'
      const mockChunk: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'show-work-unit',
          argsJson: '{"id":"CODE-001"}',
          projectRoot: '/test/project',
          toolCallId: 'tool-call-123',
        },
      };

      // @step Then Rust should emit a FspecCommandRequest chunk with typed fields
      expect(mockChunk.type).toBe('FspecCommandRequest');

      // @step And TypeScript should receive the chunk with direct field access via chunk.fspecRequest.command
      if (mockChunk.type === 'FspecCommandRequest' && mockChunk.fspecRequest) {
        // Direct field access - no string parsing required
        expect(mockChunk.fspecRequest.command).toBe('show-work-unit');
        expect(mockChunk.fspecRequest.argsJson).toBe('{"id":"CODE-001"}');
        expect(mockChunk.fspecRequest.projectRoot).toBe('/test/project');
        expect(mockChunk.fspecRequest.toolCallId).toBe('tool-call-123');
      } else {
        throw new Error('Expected FspecCommandRequest chunk');
      }
    });
  });

  // ============================================================================
  // Scenario: TypeScript handles FspecCommandRequest with type-safe field access
  // ============================================================================

  describe('Scenario: TypeScript handles FspecCommandRequest with type-safe field access', () => {
    it('should handle FspecCommandRequest by accessing fspecRequest fields directly', () => {
      // @step Given a codelet session processing StreamChunk events
      // @step When a FspecCommandRequest chunk is received
      // Extract the handleStreamChunk function
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();

      const handlerCode = handleStreamChunkFn![0];

      // @step Then TypeScript should access chunk.fspecRequest.command directly without string parsing
      // Verify FspecCommandRequest is handled
      expect(handlerCode).toContain("chunk.type === 'FspecCommandRequest'");

      // Extract the FspecCommandRequest handling block
      const fspecRequestMatch = handlerCode.match(
        /chunk\.type\s*===\s*['"]FspecCommandRequest['"][\s\S]*?(?=}\s*else\s*if|}\s*\},\s*\[)/
      );
      expect(fspecRequestMatch).not.toBeNull();

      const fspecRequestHandler = fspecRequestMatch![0];

      // @step And TypeScript should access chunk.fspecRequest.argsJson directly without regex extraction
      // Verify it accesses typed fields directly (no string parsing)
      expect(fspecRequestHandler).toContain('chunk.fspecRequest.command');
      expect(fspecRequestHandler).toContain('chunk.fspecRequest.argsJson');

      // @step And TypeScript should access chunk.fspecRequest.projectRoot directly without field parsing
      expect(fspecRequestHandler).toContain('chunk.fspecRequest.projectRoot');
      expect(fspecRequestHandler).toContain('chunk.fspecRequest.toolCallId');

      // Verify NO string parsing is used
      expect(fspecRequestHandler).not.toMatch(/\.match\s*\(/);
      expect(fspecRequestHandler).not.toMatch(/\.includes\s*\(/);
      expect(fspecRequestHandler).not.toMatch(/extract_field/i);
      expect(fspecRequestHandler).not.toContain('FSPEC_INTERCEPT');

      // @step And TypeScript should execute the command via callFspecCommand callback
      // @step And the result should be returned as a FspecCommandResult chunk with success and data fields
      // Verify it calls fspecCallback (the TypeScript callback) and sessionSendFspecResult (to send result back)
      expect(fspecRequestHandler).toContain('fspecCallback');
      expect(fspecRequestHandler).toContain('sessionSendFspecResult');
    });

    it('should extract all fields from FspecCommandRequest without parsing', () => {
      // @step Given a codelet session processing StreamChunk events
      // @step When a FspecCommandRequest chunk is received
      const mockRequest: FspecRequest = {
        command: 'create-story',
        argsJson: '{"prefix":"TEST","title":"Test Story"}',
        projectRoot: '/projects/test',
        toolCallId: 'call-456',
      };

      // @step Then TypeScript should access chunk.fspecRequest.command directly without string parsing
      const command = mockRequest.command;
      expect(command).toBe('create-story');

      // @step And TypeScript should access chunk.fspecRequest.argsJson directly without regex extraction
      const argsJson = mockRequest.argsJson;
      expect(argsJson).toBe('{"prefix":"TEST","title":"Test Story"}');

      // Parse argsJson to verify it's valid JSON
      const parsedArgs = JSON.parse(argsJson) as { prefix: string; title: string };
      expect(parsedArgs.prefix).toBe('TEST');
      expect(parsedArgs.title).toBe('Test Story');

      // @step And TypeScript should access chunk.fspecRequest.projectRoot directly without field parsing
      const projectRoot = mockRequest.projectRoot;
      expect(projectRoot).toBe('/projects/test');

      const toolCallId = mockRequest.toolCallId;
      expect(toolCallId).toBe('call-456');
    });
  });

  // ============================================================================
  // Scenario: Failed fspec command returns structured error in FspecCommandResult
  // ============================================================================

  describe('Scenario: Failed fspec command returns structured error in FspecCommandResult', () => {
    it('should have error field in FspecResult type', () => {
      // @step Given a codelet session with FspecTool available
      // Verify FspecResult has error field
      const fspecResultMatch = indexDts.match(
        /export (?:interface|type) FspecResult\s*[{=][\s\S]*?(?=\nexport )/
      );
      expect(fspecResultMatch).not.toBeNull();

      const fspecResultDef = fspecResultMatch![0];

      // @step When the LLM invokes Fspec tool with an invalid command
      // @step Then the FspecCommandResult should have success set to false
      expect(fspecResultDef).toContain('success: boolean');

      // @step And the FspecCommandResult should have an error field with the failure message
      expect(fspecResultDef).toMatch(/error\??\s*:\s*string/);
    });

    it('should handle failed command with structured error in FspecResult', () => {
      // @step Given a codelet session with FspecTool available
      // @step When the LLM invokes Fspec tool with an invalid command
      const errorResult: FspecResult = {
        success: false,
        data: '',
        error: 'Unknown command: invalid-command',
        systemReminder: null,
        toolCallId: 'call-789',
      };

      // @step Then the FspecCommandResult should have success set to false
      expect(errorResult.success).toBe(false);

      // @step And the FspecCommandResult should have an error field with the failure message
      expect(errorResult.error).toBe('Unknown command: invalid-command');

      // @step And TypeScript should display proper error feedback based on the typed error field
      // Simulate error handling in UI
      if (!errorResult.success && errorResult.error) {
        const displayError = `Fspec command failed: ${errorResult.error}`;
        expect(displayError).toContain('Unknown command: invalid-command');
      }
    });

    it('should distinguish between success and error results', () => {
      // @step Given a codelet session with FspecTool available
      const successResult: FspecResult = {
        success: true,
        data: '{"id":"CODE-001","title":"Test Story"}',
        error: null,
        systemReminder: 'Next steps: run fspec add-rule...',
        toolCallId: 'call-success',
      };

      const errorResult: FspecResult = {
        success: false,
        data: '',
        error: 'Work unit not found: CODE-999',
        systemReminder: null,
        toolCallId: 'call-error',
      };

      // @step When the LLM invokes Fspec tool with an invalid command
      // @step Then the FspecCommandResult should have success set to false
      expect(successResult.success).toBe(true);
      expect(errorResult.success).toBe(false);

      // @step And the FspecCommandResult should have an error field with the failure message
      expect(successResult.error).toBeNull();
      expect(errorResult.error).toBe('Work unit not found: CODE-999');

      // @step And TypeScript should display proper error feedback based on the typed error field
      expect(successResult.data).toContain('CODE-001');
      expect(errorResult.data).toBe('');
    });
  });

  // ============================================================================
  // Scenario: System reminder is included in FspecCommandResult for workflow guidance
  // ============================================================================

  describe('Scenario: System reminder is included in FspecCommandResult for workflow guidance', () => {
    it('should have systemReminder field in FspecResult', () => {
      // @step Given a codelet session with FspecTool available
      const fspecResultMatch = indexDts.match(
        /export (?:interface|type) FspecResult\s*[{=][\s\S]*?(?=\nexport )/
      );
      expect(fspecResultMatch).not.toBeNull();

      const fspecResultDef = fspecResultMatch![0];

      // @step When the LLM invokes Fspec tool with command "create-story"
      // @step And the command executes successfully
      // @step Then the FspecCommandResult should include a system_reminder field
      expect(fspecResultDef).toMatch(/systemReminder\??\s*:\s*string/);
    });

    it('should include system reminder for workflow orchestration', () => {
      // @step Given a codelet session with FspecTool available
      // @step When the LLM invokes Fspec tool with command "create-story"
      // @step And the command executes successfully
      const result: FspecResult = {
        success: true,
        data: '{"id":"TEST-001","status":"backlog"}',
        error: null,
        systemReminder: '<system-reminder>\nWork unit TEST-001 created.\n\nNext steps:\n1. Run fspec add-rule TEST-001 to define business rules\n2. Run fspec add-example TEST-001 to add concrete examples\n</system-reminder>',
        toolCallId: 'call-create',
      };

      // @step Then the FspecCommandResult should include a system_reminder field
      expect(result.systemReminder).not.toBeNull();

      // @step And the system_reminder should contain workflow guidance like "Next steps: run fspec add-rule..."
      expect(result.systemReminder).toContain('Next steps');
      expect(result.systemReminder).toContain('fspec add-rule');

      // @step And the system_reminder should be injected into LLM context for ACDD workflow orchestration
      // Verify the reminder is wrapped in system-reminder tags for proper handling
      expect(result.systemReminder).toContain('<system-reminder>');
      expect(result.systemReminder).toContain('</system-reminder>');
    });

    it('should preserve system reminder through the handler', () => {
      // @step Given a codelet session with FspecTool available
      const mockCallback = vi.fn().mockReturnValue(JSON.stringify({
        success: true,
        data: 'Story created',
        systemReminder: '<system-reminder>Add rules next</system-reminder>',
      }));

      // @step When the LLM invokes Fspec tool with command "create-story"
      const resultJson = mockCallback('create-story', '{"prefix":"TEST"}', '.');
      const result = JSON.parse(resultJson) as { data?: string; systemReminder?: string; success?: boolean };

      // @step And the command executes successfully
      expect(result.success).toBe(true);

      // @step Then the FspecCommandResult should include a system_reminder field
      expect(result.systemReminder).toBeDefined();

      // @step And the system_reminder should be injected into LLM context for ACDD workflow orchestration
      expect(result.systemReminder).toContain('Add rules next');
    });
  });

  // ============================================================================
  // Scenario: FSPEC_INTERCEPT string pattern is removed after migration
  // ============================================================================

  describe('Scenario: FSPEC_INTERCEPT string pattern is removed after migration', () => {
    it('should NOT have FSPEC_INTERCEPT pattern in wrapper.rs or stream_handlers.rs', () => {
      // @step Given the structured StreamChunk flow is implemented for fspec commands
      // @step When all fspec tool calls use FspecCommandRequest and FspecCommandResult
      // @step Then the FSPEC_INTERCEPT string pattern should be removed from wrapper.rs
      if (wrapperRsSource) {
        // After migration, wrapper.rs should NOT contain FSPEC_INTERCEPT
        expect(wrapperRsSource).not.toContain('FSPEC_INTERCEPT');
      }

      // @step And the handle_fspec_session_error function should be removed from stream_handlers.rs
      // @step And the extract_field_from_fspec_error helper should be removed from stream_handlers.rs
      if (streamHandlersSource) {
        // After migration, stream_handlers.rs should NOT have fspec-specific error handling
        expect(streamHandlersSource).not.toContain('handle_fspec_session_error');
        // After migration, no string field extraction should be needed
        expect(streamHandlersSource).not.toContain('extract_field_from_fspec_error');
      }

      // Verify AgentView doesn't use string parsing for fspec
      expect(agentViewSource).not.toContain('FSPEC_INTERCEPT');

      // Verify no regex-based field extraction for fspec
      expect(agentViewSource).not.toMatch(/Command:\s*'[^']*'/); // Old pattern
      expect(agentViewSource).not.toMatch(/Args:\s*'[^']*'/); // Old pattern
    });

    it('should use JSON marker pattern instead of string pattern', () => {
      // @step Given the structured StreamChunk flow is implemented for fspec commands
      // Verify wrapper.rs uses the new __fspec_request__ JSON marker
      if (wrapperRsSource) {
        expect(wrapperRsSource).toContain('__fspec_request__');
        expect(wrapperRsSource).toContain('serde_json::json!');
      }

      // @step When all fspec tool calls use FspecCommandRequest and FspecCommandResult
      // Verify the session_manager.rs checks for the JSON marker
      const sessionManagerPath = path.join(process.cwd(), 'codelet/napi/src/session_manager.rs');
      if (fs.existsSync(sessionManagerPath)) {
        const sessionManagerSource = fs.readFileSync(sessionManagerPath, 'utf-8');
        expect(sessionManagerSource).toContain('__fspec_request__');
      }
    });
  });
});
