# COV-065 research

Command used to categorise the violations:

    fspec validate-tags 2>&1 | grep -E '^\s+[A-Z]' | grep -v Suggestion \
      | sed -E 's/@[A-Za-z0-9-]+/@X/g' | sort | uniq -c | sort -rn

No source code involved; only spec/features/*.feature tags and spec/tags.json (through
`fspec register-tag`).
