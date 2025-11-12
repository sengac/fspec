# Bug Report: JIRA Research Tool Markdown Formatting Issue

**Bug ID:** BUG-076
**Date:** 2025-11-12
**Reporter:** AI Assistant
**Severity:** Medium

---

## Summary

The JIRA research tool (`fspec research --tool=jira`) outputs `[object Object]` instead of the actual description text when using the default markdown format.

---

## Environment

- **fspec Version:** 0.8.0
- **Project:** fspec
- **JIRA Instance:** https://sengac.atlassian.net
- **Platform:** darwin (macOS)

---

## Steps to Reproduce

1. Configure JIRA credentials in `~/.fspec/fspec-config.json`
2. Run the command:
   ```bash
   fspec research --tool=jira --issue CCS-6
   ```
3. Observe the output in the Description section

---

## Expected Behavior

The markdown output should display the actual description text from the JIRA issue. For example:

```markdown
## Description

The frontend interface should be user-friendly and responsive, allowing users to interact with the chatbot seamlessly.
```

---

## Actual Behavior

The markdown output displays `[object Object]` instead:

```markdown
## Description

[object Object]
```

---

## Root Cause Analysis

The JIRA API returns the description field as a structured object (Atlassian Document Format - ADF):

```json
{
  "description": {
    "type": "doc",
    "version": 1,
    "content": [
      {
        "type": "paragraph",
        "content": [
          {
            "type": "text",
            "text": "The frontend interface should be user-friendly..."
          }
        ]
      }
    ]
  }
}
```

The markdown formatter is attempting to directly convert this object to a string using JavaScript's default `toString()` method, which results in `[object Object]`.

---

## Workaround

Use the `--format json` flag to get properly formatted JSON output:

```bash
fspec research --tool=jira --issue CCS-6 --format json
```

This returns the full structured data which can be parsed correctly.

---

## Test Cases Verified

### Test Case 1: Markdown Format (FAILING)
```bash
fspec research --tool=jira --issue CCS-6
```
**Result:** Description shows `[object Object]`

### Test Case 2: JSON Format (WORKING)
```bash
fspec research --tool=jira --issue CCS-6 --format json
```
**Result:** Description properly shows ADF structure

### Test Case 3: Multiple Issues (FAILING)
All 6 issues in the FSPEC project show the same problem:
- CCS-6: Develop Frontend Interface
- CCS-5: Implement NLP Engine
- CCS-4: Design Chatbot Personality
- CCS-3: Create User Flow Diagram
- CCS-2: Chatbot Development
- CCS-1: User Interaction Design

---

## Affected Files

Based on the research tool structure, the likely affected files are:
- `spec/research-scripts/jira.js` or similar
- Markdown formatting logic for JIRA responses

---

## Suggested Fix

The markdown formatter needs to:
1. Detect that `description` is an ADF object
2. Parse the ADF content recursively to extract text
3. Convert ADF nodes to appropriate markdown format (paragraphs, lists, bold, etc.)

Example parsing logic needed:
```javascript
function parseADF(adfObject) {
  if (!adfObject || !adfObject.content) return '';

  return adfObject.content.map(node => {
    if (node.type === 'paragraph') {
      return node.content
        .map(textNode => textNode.text || '')
        .join('');
    }
    // Handle other node types: heading, bulletList, etc.
  }).join('\n\n');
}
```

---

## Impact

- **User Impact:** Medium - Users cannot read issue descriptions in markdown format
- **Workaround Available:** Yes - use `--format json`
- **Data Loss:** No - data is preserved, just not displayed correctly

---

## Related Issues

None identified yet.

---

## Additional Notes

- The JSON format works correctly, indicating the API integration is functioning
- Only the markdown rendering logic needs fixing
- Other JIRA fields (summary, status, assignee, labels) render correctly
- This affects the primary use case for the tool (human-readable markdown output)
