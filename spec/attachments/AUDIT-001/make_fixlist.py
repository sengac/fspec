#!/usr/bin/env python3
"""Generate the prioritized fix-action list from audit results."""
import json
from collections import defaultdict

per = json.load(open('per_qid.json'))
problems = json.load(open('problems.json'))['problems']

div_by_file = defaultdict(list)
for qid, a in per.items():
    if a['choice'] in ('divergent', 'unverifiable'):
        div_by_file[a['impl']].append({
            'feature': a['feature'],
            'scenario': a['scenario'],
            'verdict': a['choice'],
            'confidence': a['confidence'],
            'probabilities': a['probabilities'],
        })

out = []
for f in sorted(div_by_file, key=lambda f: -len(div_by_file[f])):
    items = sorted(div_by_file[f], key=lambda x: -x['confidence'])
    out.append({'file': f, 'count': len(items), 'items': items})

def count(kind):
    return sum(1 for p in problems if p['kind'] == kind)

fixlist = {
    'generated': '2026-09-22',
    'note': ('Action list from the 2026-09-22 laya+structural coverage audit. '
             'P0 = dead mappings (full records in problems.json, grouped by feature there). '
             'P1 = divergent/unverifiable semantic verdicts below (re-link each scenario to the '
             'specific function(s) that implement it; whole-file blob mappings are the dominant '
             'pattern). P2 = structural hygiene. Verify each fix with the coverage '
             'show/audit commands and the project validation checks.'),
    'p1_divergent_unverifiable': out,
    'p1_totals': {
        'divergent': sum(1 for a in per.values() if a['choice'] == 'divergent'),
        'unverifiable': sum(1 for a in per.values() if a['choice'] == 'unverifiable'),
    },
    'p0_dead_mappings_summary': {
        'impl_file_missing': count('impl_file_missing'),
        'test_file_missing': count('test_file_missing'),
        'impl_lines_beyond_eof': count('impl_lines_beyond_eof'),
        'test_lines_beyond_eof': count('test_lines_beyond_eof'),
    },
    'p2_hygiene': {
        'scenarios_no_test_mapping': count('scenarios_no_test_mapping'),
        'no_impl_mapping': count('no_impl_mapping'),
        'impl_suspiciously_large': count('impl_suspiciously_large'),
        'impl_scattered': count('impl_scattered'),
    },
}
json.dump(fixlist, open('fixlist.json', 'w'), indent=1)
print(f"files with p1 work: {len(out)}, p1 totals: {fixlist['p1_totals']}")
print('top 5:', [(o['file'], o['count']) for o in out[:5]])
