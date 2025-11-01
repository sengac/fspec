// Feature: spec/features/stable-indices.feature

import { describe, it, expect } from 'vitest';
import { bootstrap } from '../bootstrap';

describe('Rule 16: Bootstrap documentation for stable indices', () => {
  it('should document stable indices concept in bootstrap output', async () => {
    const output = await bootstrap();

    // Check for stable indices documentation
    expect(output.toLowerCase()).toMatch(/stable\s+(indices|ids)/);
    expect(output.toLowerCase()).toContain('soft-delete');
  });

  it('should document restore-rule command in bootstrap output', async () => {
    const output = await bootstrap();
    expect(output).toContain('restore-rule');
  });

  it('should document restore-example command in bootstrap output', async () => {
    const output = await bootstrap();
    expect(output).toContain('restore-example');
  });

  it('should document restore-question command in bootstrap output', async () => {
    const output = await bootstrap();
    expect(output).toContain('restore-question');
  });

  it('should document restore-architecture-note command in bootstrap output', async () => {
    const output = await bootstrap();
    expect(output).toContain('restore-architecture-note');
  });

  it('should document compact-work-unit command in bootstrap output', async () => {
    const output = await bootstrap();
    expect(output).toContain('compact-work-unit');
  });

  it('should document auto-compact behavior in bootstrap output', async () => {
    const output = await bootstrap();
    expect(output.toLowerCase()).toMatch(/auto[- ]compact/);
  });
});
