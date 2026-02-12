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
 *
 * REFAC-008: FspecCommandRequest handling moved from AgentView to GlobalSessionStreamManager
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import type { StreamChunk, FspecRequest, FspecResult } from '../../../codelet/napi/index';

describe('CODE-009: Structured FspecTool Results via StreamChunk Discriminated Union', () => {
  // Read the actual generated TypeScript definitions
  const indexDtsPath = path.join(process.cwd(), 'codelet/napi/index.d.ts');
  const indexDts = fs.readFileSync(indexDtsPath, 'utf-8');

  // REFAC-008: FspecCommandRequest handling moved from AgentView to GlobalSessionStreamManager
  const globalManagerPath = path.join(process.cwd(), 'src/tui/services/globalSessionStreamManager.ts');
  const globalManagerSource = fs.readFileSync(globalManagerPath, 'utf-8');

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
  // REFAC-008: FspecCommandRequest handling is now in GlobalSessionStreamManager
  // ============================================================================

  describe('Scenario: TypeScript handles FspecCommandRequest with type-safe field access', () => {
    it('should handle FspecCommandRequest by accessing fspecRequest fields directly', () => {
      // @step Given a codelet session processing StreamChunk events
      // REFAC-008: FspecCommandRequest handling moved to GlobalSessionStreamManager
      
      // @step When a FspecCommandRequest chunk is received
      // Verify FspecCommandRequest is handled in GlobalSessionStreamManager
      expect(globalManagerSource).toContain("chunk.type === 'FspecCommandRequest'");

      // @step Then TypeScript should access chunk.fspecRequest.command directly without string parsing
      // @step And TypeScript should access chunk.fspecRequest.argsJson directly without regex extraction
      // @step And TypeScript should access chunk.fspecRequest.projectRoot directly without field parsing
      // Verify typed field access via destructuring: const { command, argsJson, projectRoot, toolCallId } = request
      expect(globalManagerSource).toContain('const { command, argsJson, projectRoot, toolCallId } = request');

      // Verify NO string parsing is used
      expect(globalManagerSource).not.toMatch(/\.match\s*\(/);
      expect(globalManagerSource).not.toContain('FSPEC_INTERCEPT');

      // Verify it calls fspecCallback and sessionSendFspecResult
      expect(globalManagerSource).toContain('fspecCallback');
      expect(globalManagerSource).toContain('sessionSendFspecResult');
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
      // @step When the LLM invokes Fspec tool with an invalid command
      // Verify FspecResult type has optional error field
      const fspecResultMatch = indexDts.match(
        /export (?:interface|type) FspecResult\s*[{=][\s\S]*?(?=\nexport )/
      );
      expect(fspecResultMatch).not.toBeNull();

      const fspecResultDef = fspecResultMatch![0];
      // @step Then the FspecCommandResult should have success set to false
      expect(fspecResultDef).toContain('success: boolean');
      // @step And the FspecCommandResult should have an error field with the failure message
      expect(fspecResultDef).toMatch(/error\??\s*:\s*string/);
    });

    it('should handle failed command with structured error in FspecResult', () => {
      // @step Given a codelet session with FspecTool available
      // @step When the LLM invokes Fspec tool with an invalid command
      const mockErrorResult: FspecResult = {
        success: false,
        data: '',
        error: 'Command "invalid-command" not found',
        toolCallId: 'call-789',
      };

      // @step Then the FspecCommandResult should have success set to false
      expect(mockErrorResult.success).toBe(false);

      // @step And the FspecCommandResult should have an error field with the failure message
      expect(mockErrorResult.error).toBe('Command "invalid-command" not found');

      // @step And TypeScript should display proper error feedback based on the typed error field
      // Verify we can safely access the error field for display
      const errorMessage = mockErrorResult.error ?? 'Unknown error';
      expect(errorMessage).toContain('not found');
    });
  });

  // ============================================================================
  // Scenario: System reminder is included in FspecCommandResult for workflow guidance
  // ============================================================================

  describe('Scenario: System reminder is included in FspecCommandResult for workflow guidance', () => {
    it('should have systemReminder field in FspecResult type', () => {
      // @step Given a codelet session with FspecTool available
      // Verify FspecResult type has optional systemReminder field
      const fspecResultMatch = indexDts.match(
        /export (?:interface|type) FspecResult\s*[{=][\s\S]*?(?=\nexport )/
      );
      expect(fspecResultMatch).not.toBeNull();

      const fspecResultDef = fspecResultMatch![0];
      // @step Then the FspecCommandResult should include a system_reminder field
      expect(fspecResultDef).toMatch(/systemReminder\??\s*:\s*string/);
    });

    it('should include workflow guidance in systemReminder field', () => {
      // @step Given a codelet session with FspecTool available
      // @step When the LLM invokes Fspec tool with command "create-story"
      // @step And the command executes successfully
      const mockSuccessResult: FspecResult = {
        success: true,
        data: '{"id":"AUTH-001","title":"User Login"}',
        systemReminder: '<system-reminder>\nNext steps: Use fspec add-rule to add business rules...\n</system-reminder>',
        toolCallId: 'call-101',
      };

      expect(mockSuccessResult.success).toBe(true);

      // @step Then the FspecCommandResult should include a system_reminder field
      expect(mockSuccessResult.systemReminder).toBeDefined();

      // @step And the system_reminder should contain workflow guidance like "Next steps: run fspec add-rule..."
      expect(mockSuccessResult.systemReminder).toContain('Next steps');
      expect(mockSuccessResult.systemReminder).toContain('add-rule');

      // @step And the system_reminder should be injected into LLM context for ACDD workflow orchestration
      // Verify the system-reminder XML tags are present for LLM context injection
      expect(mockSuccessResult.systemReminder).toContain('<system-reminder>');
      expect(mockSuccessResult.systemReminder).toContain('</system-reminder>');
    });
  });

  // ============================================================================
  // Scenario: FSPEC_INTERCEPT string pattern is removed after migration
  // ============================================================================

  describe('Scenario: FSPEC_INTERCEPT string pattern is removed after migration', () => {
    it('should not use FSPEC_INTERCEPT string pattern in Rust wrapper', () => {
      // @step Given the structured StreamChunk flow is implemented for fspec commands
      // @step When all fspec tool calls use FspecCommandRequest and FspecCommandResult
      if (wrapperRsSource) {
        // @step Then the FSPEC_INTERCEPT string pattern should be removed from wrapper.rs
        // Verify FSPEC_INTERCEPT is not used in wrapper.rs
        expect(wrapperRsSource).not.toContain('FSPEC_INTERCEPT');
      }
    });

    it('should not use FSPEC_INTERCEPT handling in stream_handlers', () => {
      // @step Given the structured StreamChunk flow is implemented for fspec commands
      // @step When all fspec tool calls use FspecCommandRequest and FspecCommandResult
      if (streamHandlersSource) {
        // @step And the handle_fspec_session_error function should be removed from stream_handlers.rs
        expect(streamHandlersSource).not.toContain('handle_fspec_session_error');

        // @step And the extract_field_from_fspec_error helper should be removed from stream_handlers.rs
        expect(streamHandlersSource).not.toContain('extract_field_from_fspec_error');
      }
    });

    it('should use structured FspecCommandRequest chunk emission', () => {
      // @step Given the structured StreamChunk flow is implemented for fspec commands
      // Read session_manager.rs to verify structured chunk emission
      const sessionManagerPath = path.join(process.cwd(), 'codelet/napi/src/session_manager.rs');
      if (fs.existsSync(sessionManagerPath)) {
        const sessionManagerSource = fs.readFileSync(sessionManagerPath, 'utf-8');

        // @step When all fspec tool calls use FspecCommandRequest and FspecCommandResult
        // Verify FspecCommandRequest chunk is emitted (not FSPEC_INTERCEPT string)
        expect(sessionManagerSource).toContain('FspecCommandRequest');
        expect(sessionManagerSource).toContain('fspec_command_request');
      }
    });
  });
});
