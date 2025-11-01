// Feature: spec/features/stable-indices.feature

import { describe, it, expect } from 'vitest';
import { getWorkHelpContent } from '../help';

describe('Rule 17: Help.ts documentation for stable indices', () => {
  it('should document stable indices concept in work help', () => {
    const output = getWorkHelpContent();

    // Check for stable indices documentation
    expect(output.toLowerCase()).toMatch(/stable\s+(indices|ids)/);
  });

  it('should document soft-delete pattern in work help', () => {
    const output = getWorkHelpContent();
    expect(output.toLowerCase()).toContain('soft-delete');
  });

  it('should document restore-rule command in work help', () => {
    const output = getWorkHelpContent();
    expect(output).toContain('restore-rule');
  });

  it('should document restore-example command in work help', () => {
    const output = getWorkHelpContent();
    expect(output).toContain('restore-example');
  });

  it('should document restore-question command in work help', () => {
    const output = getWorkHelpContent();
    expect(output).toContain('restore-question');
  });

  it('should document restore-architecture-note command in work help', () => {
    const output = getWorkHelpContent();
    expect(output).toContain('restore-architecture-note');
  });

  it('should document compact-work-unit command in work help', () => {
    const output = getWorkHelpContent();
    expect(output).toContain('compact-work-unit');
  });

  it('should document show-deleted command in work help', () => {
    const output = getWorkHelpContent();
    expect(output).toContain('show-deleted');
  });

  it('should explain that indices are shown in display output', () => {
    const output = getWorkHelpContent();
    // Check that help mentions IDs being displayed or shown
    expect(output.toLowerCase()).toMatch(
      /(indices|ids).*(shown|displayed|shown in|display)/
    );
  });
});
