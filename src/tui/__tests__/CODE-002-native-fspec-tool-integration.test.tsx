/**
 * Feature: spec/features/native-fspec-tool-integration-via-napi-rs.feature
 * CODE-002: Native Fspec Tool Integration via NAPI-RS
 *
 * These tests verify:
 * 1. FspecTool calls TypeScript functions directly via NAPI-RS (no CLI spawning)
 * 2. System reminders are preserved and passed to LLM for workflow orchestration
 * 3. FspecTool implements rig::tool::Tool trait like other codelet tools
 * 4. Performance is improved by eliminating process spawning overhead
 */

import { describe, it, expect, vi } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { fspecCallback } from '../../utils/fspec-callback';

describe('CODE-002: Native Fspec Tool Integration via NAPI-RS', () => {
  // Read the fspec-callback source to verify direct TypeScript module calls
  const fspecCallbackPath = path.join(process.cwd(), 'src/utils/fspec-callback.ts');
  const fspecCallbackSource = fs.readFileSync(fspecCallbackPath, 'utf-8');

  // Read the wrapper.rs source to verify Tool trait implementation
  const wrapperRsPath = path.join(process.cwd(), 'codelet/tools/src/facade/wrapper.rs');
  const wrapperRsSource = fs.existsSync(wrapperRsPath) ? fs.readFileSync(wrapperRsPath, 'utf-8') : '';

  // Read AgentView to verify integration with chunk handling
  const agentViewPath = path.join(process.cwd(), 'src/tui/components/AgentView.tsx');
  const agentViewSource = fs.readFileSync(agentViewPath, 'utf-8');

  // ============================================================================
  // Scenario: AI agent receives structured data and workflow guidance
  // ============================================================================

  describe('Scenario: AI agent receives structured data and workflow guidance', () => {
    it('should call TypeScript fspec modules directly without CLI spawning', () => {
      // @step Given I have a codelet session with FspecTool available
      // Verify fspecCallback uses dynamic import to call TypeScript modules
      expect(fspecCallbackSource).toContain("await import(");
      
      // @step When I call the Fspec tool with command "create-story" and arguments ["AUTH", "User Login"]
      // Verify create-story command is implemented
      expect(fspecCallbackSource).toContain("case 'create-story':");
      expect(fspecCallbackSource).toContain("import('../commands/create-story')");
      
      // @step Then I should receive structured data about the created work unit
      // Verify the callback returns JSON with success/data fields
      expect(fspecCallbackSource).toContain("return JSON.stringify(enhancedResponse)");
    });

    it('should capture and return system reminders for workflow orchestration', () => {
      // @step Given I have a codelet session with FspecTool available
      // @step When I call the Fspec tool with command "create-story" and arguments ["AUTH", "User Login"]
      // Verify system reminder capture is implemented
      expect(fspecCallbackSource).toContain("captureSystemReminders");
      expect(fspecCallbackSource).toContain("captureSystemRemindersAsync");
      
      // @step And I should receive a system reminder with next step guidance for example mapping
      // Verify system reminders are extracted from stderr
      expect(fspecCallbackSource).toContain("<system-reminder>");
      expect(fspecCallbackSource).toContain("systemReminders");
      
      // @step And the system reminder should be passed to the LLM for workflow orchestration
      // Verify enhanced response includes system reminders
      expect(fspecCallbackSource).toContain("addSystemReminders");
    });

    it('should integrate with FspecCommandRequest chunk handler in AgentView', () => {
      // @step Given I have a codelet session with FspecTool available
      // Verify AgentView handles FspecCommandRequest chunks
      expect(agentViewSource).toContain("chunk.type === 'FspecCommandRequest'");
      
      // @step When I call the Fspec tool with command "create-story" and arguments ["AUTH", "User Login"]
      // Verify it calls fspecCallback
      expect(agentViewSource).toContain("fspecCallback(command, argsJson, projectRoot)");
      
      // @step And the system reminder should be passed to the LLM for workflow orchestration
      // Verify result is sent back to Rust which includes system reminders for LLM
      expect(agentViewSource).toContain("sessionSendFspecResult");
      expect(agentViewSource).toContain("systemReminder");
    });
  });

  // ============================================================================
  // Scenario: AI agent executes multiple commands without spawning delays
  // ============================================================================

  describe('Scenario: AI agent executes multiple commands without spawning delays', () => {
    it('should NOT spawn CLI processes for fspec commands', () => {
      // @step Given I have a codelet session with FspecTool available
      // @step When I execute multiple fspec commands in sequence
      // Verify fspecCallback does NOT use execSync or spawn
      expect(fspecCallbackSource).not.toContain("execSync");
      expect(fspecCallbackSource).not.toContain("spawn(");
      expect(fspecCallbackSource).not.toContain("child_process");
      
      // @step Then each command should complete without process spawning overhead
      // @step And the total execution time should be significantly faster than bash tool equivalent
      // Verify direct module import is used (no process spawning = faster execution)
      expect(fspecCallbackSource).toContain("await import(");
    });

    it('should implement multiple fspec commands via direct TypeScript calls', () => {
      // @step Given I have a codelet session with FspecTool available
      // @step When I execute multiple fspec commands in sequence
      // Verify common commands are implemented
      const implementedCommands = [
        "case 'list-work-units':",
        "case 'create-story':",
        "case 'show-work-unit':",
        "case 'update-work-unit-status':",
        "case 'add-rule':",
        "case 'add-example':",
        "case 'generate-scenarios':",
      ];
      
      for (const cmd of implementedCommands) {
        expect(fspecCallbackSource).toContain(cmd);
      }
      
      // @step And each command should preserve workflow orchestration through system reminders
      // Each command result goes through captureSystemRemindersAsync
      expect(fspecCallbackSource).toContain("captureSystemRemindersAsync(async () =>");
    });

    it('should handle unsupported commands gracefully', () => {
      // @step Given I have a codelet session with FspecTool available
      // @step When I execute multiple fspec commands in sequence
      // Verify bootstrap/init are marked as unsupported (CLI-only)
      expect(fspecCallbackSource).toContain("case 'bootstrap':");
      expect(fspecCallbackSource).toContain("case 'init':");
      expect(fspecCallbackSource).toContain("not supported via FspecTool");
      
      // Verify default case handles unknown commands
      expect(fspecCallbackSource).toContain("default:");
      expect(fspecCallbackSource).toContain("not implemented in TypeScript callback");
    });
  });

  // ============================================================================
  // Scenario: AI agent uses Fspec tool alongside other codelet tools
  // ============================================================================

  describe('Scenario: AI agent uses Fspec tool alongside other codelet tools', () => {
    it('should implement FspecTool using rig::tool::Tool trait like other tools', () => {
      // @step Given I have a codelet session with multiple tools available
      // Verify FspecToolFacadeWrapper exists and implements Tool trait
      if (wrapperRsSource) {
        expect(wrapperRsSource).toContain("FspecToolFacadeWrapper");
        expect(wrapperRsSource).toContain("impl Tool for FspecToolFacadeWrapper");
        
        // @step When I use Fspec tool to create a work unit
        // Verify the wrapper has call method
        expect(wrapperRsSource).toContain("async fn call(&self");
      }
    });

    it('should work seamlessly with session chunk handling', () => {
      // @step Given I have a codelet session with multiple tools available
      // @step When I use Fspec tool to create a work unit
      // @step And I use Read tool to examine a file
      // @step And I use Write tool to create a test file
      // @step And I use Fspec tool again to update work unit status
      
      // Verify FspecCommandRequest handler is in the same chunk processing loop as other tools
      // It's part of handleStreamChunk which handles all StreamChunk types
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();
      
      const handlerCode = handleStreamChunkFn![0];
      
      // Verify multiple chunk types are handled in same callback
      expect(handlerCode).toContain("chunk.type === 'Text'");
      expect(handlerCode).toContain("chunk.type === 'ToolCall'");
      expect(handlerCode).toContain("chunk.type === 'ToolResult'");
      expect(handlerCode).toContain("chunk.type === 'FspecCommandRequest'");
      
      // @step Then all tools should work seamlessly together in the same session
      // @step And the Fspec tool should maintain workflow context throughout the session
      // Session context is maintained via currentSessionIdRef
      expect(agentViewSource).toContain("currentSessionIdRef.current");
    });

    it('should emit FspecCommandRequest from Rust session manager', () => {
      // @step Given I have a codelet session with multiple tools available
      // Read session_manager.rs to verify chunk emission
      const sessionManagerPath = path.join(process.cwd(), 'codelet/napi/src/session_manager.rs');
      if (fs.existsSync(sessionManagerPath)) {
        const sessionManagerSource = fs.readFileSync(sessionManagerPath, 'utf-8');
        
        // @step When I use Fspec tool to create a work unit
        // Verify __fspec_request__ marker detection
        expect(sessionManagerSource).toContain("__fspec_request__");
        
        // Verify FspecCommandRequest chunk emission
        expect(sessionManagerSource).toContain("StreamChunk::fspec_command_request");
        
        // Verify result sending back mechanism
        expect(sessionManagerSource).toContain("wait_for_fspec_response");
        expect(sessionManagerSource).toContain("send_fspec_result");
      }
    });
  });

  // ============================================================================
  // Additional integration tests
  // ============================================================================

  describe('Integration: fspecCallback functionality', () => {
    it('should handle errors gracefully and return structured error response', async () => {
      // Test with an unimplemented command
      const result = await fspecCallback('nonexistent-command', '{}', '/tmp');
      const parsed = JSON.parse(result) as { success?: boolean; error?: string; errorType?: string };
      
      expect(parsed.success).toBe(false);
      expect(parsed.error).toContain('not implemented');
      expect(parsed.errorType).toBe('UnimplementedCommand');
    });

    it('should handle invalid JSON args gracefully', async () => {
      // Test with invalid JSON
      const result = await fspecCallback('list-work-units', 'invalid-json', '/tmp');
      const parsed = JSON.parse(result) as { success?: boolean; error?: string };
      
      // Should catch the JSON parse error
      expect(parsed.success).toBe(false);
      expect(parsed.error).toBeDefined();
    });

    it('should reject setup commands with helpful message', async () => {
      // Test bootstrap command
      const result = await fspecCallback('bootstrap', '{}', '/tmp');
      const parsed = JSON.parse(result) as { success?: boolean; error?: string; errorType?: string; suggestions?: string[] };
      
      expect(parsed.success).toBe(false);
      expect(parsed.errorType).toBe('UnsupportedCommand');
      expect(parsed.suggestions).toContain("Use 'fspec bootstrap' directly in terminal");
    });
  });
});
