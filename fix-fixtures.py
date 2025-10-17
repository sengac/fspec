#!/usr/bin/env python3
"""
Script to replace old foundation schema fixtures with generic schema v2.0.0 fixtures
"""

import re
import sys

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Pattern to match the old foundation fixture (multi-line object literal)
    # We'll look for the pattern: const minimalFoundation = { ... };
    # and replace with: const minimalFoundation = createMinimalFoundation();

    # This pattern matches from "const minimalFoundation = {" to the closing "};"
    # accounting for nested braces
    pattern = r'const minimalFoundation = \{[^}]*?\$schema:.*?notes: \{ developmentStatus: \[\] \},\s*\};'

    # Replace with helper function call
    replacement = 'const minimalFoundation = createMinimalFoundation();'

    # Use DOTALL flag to match across newlines
    content = re.sub(pattern, replacement, content, flags=re.DOTALL)

    # Also replace foundationData fixtures
    pattern2 = r'const foundationData = \{[^}]*?\$schema:.*?notes: \{ developmentStatus: \[\] \},\s*\};'
    replacement2 = 'const foundationData = createMinimalFoundation();'
    content = re.sub(pattern2, replacement2, content, flags=re.DOTALL)

    # Update field path references in expectations
    # whatWeAreBuilding.projectOverview -> solutionSpace.overview
    # whyWeAreBuildingIt.problemDefinition.primary.description -> problemSpace.primaryProblem.description

    content = content.replace('whatWeAreBuilding.projectOverview', 'solutionSpace.overview')
    content = content.replace('whyWeAreBuildingIt.problemDefinition.primary.description', 'problemSpace.primaryProblem.description')
    content = content.replace('whatWeAreBuilding', 'solutionSpace')
    content = content.replace('whyWeAreBuildingIt', 'problemSpace')

    with open(filepath, 'w') as f:
        f.write(content)

    print(f"Fixed {filepath}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: fix-fixtures.py <file1> [file2] [file3] ...")
        sys.exit(1)

    for filepath in sys.argv[1:]:
        try:
            fix_file(filepath)
        except Exception as e:
            print(f"Error fixing {filepath}: {e}")
            sys.exit(1)
