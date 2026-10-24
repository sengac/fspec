import { readFileSync } from 'node:fs';

const names = [
  'codex-buffering-wire-compatibility',
  'codex-metadata-presentation',
  'transient-provider-waiting',
  'codex-metadata-safety-status',
  'codex-session-metadata-status',
];
let total = 0;
for (const name of names) {
  const feature = readFileSync(`spec/features/${name}.feature`, 'utf8');
  const coverage = JSON.parse(readFileSync(`spec/features/${name}.feature.coverage`, 'utf8'));
  const scenarios = [...feature.matchAll(/^  Scenario: (.+)\n([\s\S]*?)(?=^  Scenario: |$(?![\s\S]))/gm)];
  if (scenarios.length !== coverage.scenarios.length) throw new Error(`${name}: scenario count mismatch`);
  for (const [, title, body] of scenarios) {
    const mapping = coverage.scenarios.find(scenario => scenario.name === title);
    if (mapping?.testMappings.length !== 1) throw new Error(`${title}: expected exactly one test mapping`);
    const test = mapping.testMappings[0];
    const [start, end] = test.lines.split('-').map(Number);
    const source = readFileSync(test.file, 'utf8').split('\n');
    if (!(start >= 1 && end >= start && end <= source.length)) throw new Error(`${title}: invalid test range`);
    const slice = source.slice(start - 1, end).join('\n');
    const tests = [...slice.matchAll(/#\[(?:tokio::)?test(?:\([^\n]*\))?\]/g)];
    if (tests.length !== 1) throw new Error(`${title}: test range is not 1:1`);
    for (const [, keyword, text] of body.matchAll(/^    (Given|When|Then|And|But) (.+)$/gm)) {
      if (!slice.includes(`// @step ${keyword} ${text}`)) throw new Error(`${title}: missing step ${keyword} ${text}`);
    }
    if (!test.implMappings?.length) throw new Error(`${title}: missing implementation links`);
    for (const implementation of test.implMappings) {
      const length = readFileSync(implementation.file, 'utf8').split('\n').length;
      if (!implementation.lines.length || implementation.lines.some(line => !Number.isInteger(line) || line < 1 || line > length)) {
        throw new Error(`${title}: invalid implementation line`);
      }
    }
    total++;
  }
  console.log(`${name}: ${scenarios.length} unique scenario tests with exact steps and valid implementation links`);
}
console.log(`PASS: ${total}/${total} scenario mappings audited (traceability, not instrumented code coverage)`);
