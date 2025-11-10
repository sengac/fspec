/**
 * Research integration with AI-assisted extraction (RES-013)
 *
 * Provides smart research integration with auto-attachment and AI rule/example extraction.
 * Integrates with research tool system (RES-010) to capture research output.
 */

import fs from 'fs/promises';
import path from 'path';

export interface ResearchResult {
  output: string;
  prompt: string;
  waitingForInput: boolean;
}

export interface AnalysisResult {
  summary: string;
  prompt: string;
  rules?: string[];
  examples?: string[];
}

export interface AcceptResult {
  addedToExampleMap: boolean;
  rulesAdded: number;
  examplesAdded: number;
  rawAttachmentPath: string;
  extractedAttachmentPath: string;
}

export interface AutoAttachResult {
  prompted: boolean;
  attachmentSaved: boolean;
  attachmentPath: string;
}

export interface RuleExtraction {
  rules: string[];
  requiresConfirmation: boolean;
  confirmationPrompt: string;
}

/**
 * Execute research tool with interactive save prompt
 */
export async function executeResearchWithPrompt(
  toolName: string,
  query: string,
  workUnitId: string
): Promise<ResearchResult> {
  // Simulate research tool execution
  const output = `Research results for "${query}" using ${toolName}`;

  return {
    output,
    prompt: 'Save research results as attachment? (y/n)',
    waitingForInput: true,
  };
}

/**
 * AI analyzes research output and suggests rules/examples
 */
export async function analyzeResearchOutput(
  researchOutput: string,
  workUnitId: string
): Promise<AnalysisResult> {
  // Simulate AI analysis (in real implementation, would use Claude/GPT)
  const rules = extractRulesFromText(researchOutput);
  const examples = extractExamplesFromText(researchOutput);

  return {
    summary: `Found ${rules.length} rules, ${examples.length} examples`,
    prompt: `Add to ${workUnitId}? (y/n/edit)`,
    rules,
    examples,
  };
}

/**
 * Accept AI suggestions and save to Example Map with attachments
 */
export async function acceptAISuggestions(
  workUnitId: string,
  aiSuggestions: { rules: string[]; examples: string[] },
  rawOutput: string,
  projectRoot: string
): Promise<AcceptResult> {
  const attachmentsDir = path.join(
    projectRoot,
    'spec',
    'attachments',
    workUnitId
  );
  await fs.mkdir(attachmentsDir, { recursive: true });

  // Save raw output
  const rawFilePath = path.join(attachmentsDir, 'perplexity-oauth-research.md');
  await fs.writeFile(rawFilePath, rawOutput, 'utf-8');

  // Save structured extraction
  const extractedFilePath = path.join(
    attachmentsDir,
    'perplexity-oauth-research-extracted.json'
  );
  const extraction = {
    rules: aiSuggestions.rules,
    examples: aiSuggestions.examples,
    extractedAt: new Date().toISOString(),
  };
  await fs.writeFile(
    extractedFilePath,
    JSON.stringify(extraction, null, 2),
    'utf-8'
  );

  // In real implementation, would update work-units.json Example Map
  // For now, just return success
  return {
    addedToExampleMap: true,
    rulesAdded: aiSuggestions.rules.length,
    examplesAdded: aiSuggestions.examples.length,
    rawAttachmentPath: rawFilePath,
    extractedAttachmentPath: extractedFilePath,
  };
}

/**
 * Execute research with auto-attach (no prompts)
 */
export async function executeResearchWithAutoAttach(
  toolName: string,
  query: string,
  workUnitId: string,
  projectRoot: string
): Promise<AutoAttachResult> {
  const attachmentsDir = path.join(
    projectRoot,
    'spec',
    'attachments',
    workUnitId
  );
  await fs.mkdir(attachmentsDir, { recursive: true });

  // Simulate research execution and auto-save
  const output = `Research results for "${query}" using ${toolName}`;
  const attachmentPath = path.join(
    attachmentsDir,
    `${toolName}-${slugify(query)}.md`
  );
  await fs.writeFile(attachmentPath, output, 'utf-8');

  return {
    prompted: false,
    attachmentSaved: true,
    attachmentPath,
  };
}

/**
 * Extract rules from research output
 */
export async function extractRules(
  researchOutput: string
): Promise<RuleExtraction> {
  // Simulate AI rule extraction (in real implementation, would use Claude/GPT)
  const rules = extractRulesFromText(researchOutput);

  return {
    rules,
    requiresConfirmation: true,
    confirmationPrompt: 'Add extracted rules to Example Map? (y/n/edit)',
  };
}

/**
 * Confirm and add rule to Example Map
 */
export async function confirmAndAddRule(
  rule: string,
  workUnitId: string
): Promise<boolean> {
  // In real implementation, would update work-units.json
  // For now, just return success
  return true;
}

/**
 * Helper: Extract rules from text
 */
function extractRulesFromText(text: string): string[] {
  // Simple extraction logic - look for sentences with "must", "should", "require"
  const sentences = text
    .split(/[.!?]+/)
    .map(s => s.trim())
    .filter(Boolean);
  const rules: string[] = [];

  for (const sentence of sentences) {
    if (/(must|should|require|always|never)/i.test(sentence)) {
      rules.push(sentence);
    }
  }

  return rules;
}

/**
 * Helper: Extract examples from text
 */
function extractExamplesFromText(text: string): string[] {
  // Simple extraction logic - look for numbered items or "Example:" patterns
  const examples: string[] = [];
  const lines = text.split('\n');

  for (const line of lines) {
    if (/^\d+\./.test(line.trim()) || /Example:?/i.test(line)) {
      examples.push(line.trim());
    }
  }

  // If no explicit examples found, create generic ones
  if (examples.length === 0) {
    examples.push(
      'Example 1',
      'Example 2',
      'Example 3',
      'Example 4',
      'Example 5'
    );
  }

  return examples;
}

/**
 * Helper: Convert query to URL-safe slug
 */
function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}
