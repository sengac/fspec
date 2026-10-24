import hashlib
import json
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
OWNED_TAGS = re.compile(r'^@CMPCT-(060|061|062|067|068)$', re.M)
EXCLUDED = {'runtime-uncommitted-context-publication'}


def functions(path):
    lines = (ROOT / path).read_text().splitlines()
    result = []
    for index, line in enumerate(lines):
        match = re.match(r'( *)(?:pub(?:\([^)]*\))? )?(?:async )?fn (\w+)', line)
        if match is None:
            continue
        indent, name = match.groups()
        end = next((row for row in range(index + 1, len(lines))
                    if lines[row] == indent + '}'), None)
        if end is None:
            continue
        start = index
        while start and lines[start - 1].lstrip().startswith(('#[', '///')):
            start -= 1
        body = '\n'.join(lines[start:end + 1])
        if re.search(r'#\[(?:tokio::)?test', body):
            result.append((name, start + 1, end + 1, body))
    return result


def scenarios(text):
    result = []
    for match in re.finditer(r'^  Scenario( Outline)?: (.+)$', text, re.M):
        tail = text[match.end():]
        end = re.search(r'^  Scenario(?: Outline)?: ', tail, re.M)
        body = tail[:end.start()] if end else tail
        steps = re.findall(r'^    ((?:Given|When|Then|And|But) .+)$', body, re.M)
        rows = re.findall(r'^      \|.+\|\s*$', body, re.M)
        count = len(rows) - 1 if match.group(1) else 1
        result.append((match.group(2), steps, count))
    return result


def command(arguments):
    completed = subprocess.run(['fspec', *arguments], cwd=ROOT,
                               text=True, capture_output=True)
    assert completed.returncode == 0, (arguments, completed.stdout, completed.stderr)
    return {'arguments': arguments, 'exit': completed.returncode,
            'stdout': completed.stdout, 'stderr': completed.stderr}


def audit(path):
    text = path.read_text()
    coverage_path = path.with_suffix('.feature.coverage')
    coverage = json.loads(coverage_path.read_text())
    expected = scenarios(text)
    assert [entry['name'] for entry in coverage['scenarios']] == [
        name for name, _, _ in expected], path
    files = {path, coverage_path}
    test_files = set()
    impl_files = set()
    declared_lines = 0
    mapped = []
    for scenario, (name, steps, count) in zip(coverage['scenarios'], expected):
        tests = scenario['testMappings']
        assert len(tests) == count, (path, name, len(tests), count)
        identities = set()
        for test in tests:
            identity = (test['file'], test['lines'])
            assert identity not in identities, (path, name, identity)
            identities.add(identity)
            start, end = map(int, test['lines'].split('-'))
            matches = [entry for entry in functions(test['file'])
                       if entry[1:3] == (start, end)]
            assert len(matches) == 1, (path, name, identity, 'not a complete test')
            function_name, _, _, body = matches[0]
            comments = re.findall(r'@step\s+(.+)', body)
            assert set(comments) == set(steps), (
                path, name, function_name, set(steps) - set(comments),
                set(comments) - set(steps))
            assert len(comments) == len(set(comments)), (
                path, name, function_name, 'duplicate step annotations')
            assert test['implMappings'], (path, name, 'missing implementation')
            declared_lines += end - start + 1
            test_files.add(test['file'])
            files.add(ROOT / test['file'])
            for implementation in test['implMappings']:
                impl_path = ROOT / implementation['file']
                impl_lines = implementation['lines']
                assert isinstance(impl_lines, list) and impl_lines, impl_path
                assert len(impl_lines) == len(set(impl_lines)), impl_path
                assert min(impl_lines) >= 1, impl_path
                assert max(impl_lines) <= len(impl_path.read_text().splitlines()), impl_path
                assert '/tests/' not in implementation['file'], impl_path
                declared_lines += len(impl_lines)
                impl_files.add(implementation['file'])
                files.add(impl_path)
            mapped.append({'scenario': name, 'test': function_name,
                           'file': test['file'], 'lines': test['lines']})
    stats = coverage['stats']
    assert stats['totalScenarios'] == len(expected), (path, stats)
    assert stats['coveredScenarios'] == len(expected), (path, stats)
    assert stats['coveragePercent'] == 100, (path, stats)
    assert set(stats['testFiles']) == test_files, path
    assert set(stats['implFiles']) == impl_files, path
    assert stats['totalLinesCovered'] == declared_lines, (path, stats, declared_lines)
    checks = [command(['validate', str(path.relative_to(ROOT))]),
              command(['validate-tags', str(path.relative_to(ROOT))]),
              command(['audit-coverage', path.stem])]
    return {'feature': str(path.relative_to(ROOT)), 'mappings': mapped,
            'checks': checks, 'sha256': {
                str(file.relative_to(ROOT)): hashlib.sha256(file.read_bytes()).hexdigest()
                for file in sorted(files)}}


features = [path for path in sorted((ROOT / 'spec/features').glob('*.feature'))
            if OWNED_TAGS.search(path.read_text()) and path.stem not in EXCLUDED]
results = [audit(path) for path in features]
print(json.dumps({'features': len(results),
                  'tests': sum(len(entry['mappings']) for entry in results),
                  'results': results}, indent=2))
