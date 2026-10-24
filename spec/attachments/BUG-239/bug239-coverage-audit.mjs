import { readFileSync } from 'node:fs';

const names = ['privacy-conscious-diagnostic-logging', 'privacy-diagnostic-header-and-provider-errors'];
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
    if ([...slice.matchAll(/(?:async )?fn scenario_/g)].length !== 1) throw new Error(`${title}: test range is not 1:1`);
    for (const [, keyword, text] of body.matchAll(/^    (Given|When|Then|And|But) (.+)$/gm)) {
      if (!slice.includes(`// @step ${keyword} ${text}`)) throw new Error(`${title}: missing step ${keyword} ${text}`);
    }
    if (!test.implMappings?.length) throw new Error(`${title}: missing implementation links`);
    for (const implementation of test.implMappings) {
      const length = readFileSync(implementation.file, 'utf8').split('\n').length;
      if (implementation.lines.some(line => !Number.isInteger(line) || line < 1 || line > length)) {
        throw new Error(`${title}: invalid implementation line`);
      }
    }
    total++;
  }
  console.log(`${name}: ${scenarios.length} scenarios have unique test ranges, exact step annotations, and in-bounds implementation links`);
}
console.log(`PASS: ${total}/${total} scenario mappings audited`);
