# fspec Exit Code Bug - Findings

## Issue Summary

The `fspec add-domain-event` command (and likely other similar commands) exits with code 1 even when successfully completing its operation. This causes issues when chaining commands with `&&` and creates confusion about whether the command succeeded.

## Steps to Reproduce

1. Create a work unit and start Event Storm:
   ```bash
   fspec create-story UI "Test Story"
   fspec update-work-unit-status UI-001 specifying
   fspec discover-event-storm UI-001
   ```

2. Add a domain event:
   ```bash
   fspec add-domain-event UI-001 "TestEvent"
   echo "Exit code: $?"
   ```

3. Observe:
   - Exit code is 1 (error)
   - But the event WAS successfully added (verified via `fspec show-event-storm UI-001`)

## Expected Behavior

- Command should exit with code 0 on success
- Command should exit with code 1 only on actual errors (invalid args, work unit not found, etc.)

## Actual Behavior

- Command exits with code 1 even on successful execution
- Event is added to work-units.json correctly
- No error message is displayed (just "Exit code 1")

## Impact

### High Priority Issues:

1. **Command Chaining Broken**: Cannot use `&&` to chain multiple commands
   ```bash
   # This stops after first command:
   fspec add-domain-event UI-001 "Event1" && fspec add-domain-event UI-001 "Event2"
   ```

2. **CI/CD Pipeline Failures**: Any automation using these commands will fail in CI pipelines that check exit codes

3. **User Confusion**: Users think commands failed when they actually succeeded

### Workarounds Currently Used:

- Use `;` instead of `&&` (but this continues even on real errors)
- Ignore exit codes entirely (dangerous - masks real failures)
- Run commands one-by-one and verify manually

## Commands Affected

Based on testing, likely affected commands include:
- `fspec add-domain-event`
- `fspec add-command`
- `fspec add-policy`
- `fspec add-hotspot`
- Possibly other Event Storm commands

## Evidence

### Test Session Output:

```bash
$ fspec add-domain-event UI-005 "TrackPaused"
<no output>
$ echo $?
1

$ fspec show-event-storm UI-005 | jq -r '.[] | select(.text=="TrackPaused")'
{
  "id": 2,
  "type": "event",
  "color": "orange",
  "text": "TrackPaused",
  "deleted": false,
  "createdAt": "2025-11-18T05:22:15.123Z"
}
```

Event was successfully added despite exit code 1.

## Additional Context

- Tested on: macOS (Darwin 24.6.0)
- fspec version: Latest (installed via npm)
- Node.js version: v22.20.0
- Shell: bash

## Suggested Fix

1. Review exit code handling in Event Storm command implementations
2. Ensure commands return:
   - `process.exit(0)` or implicit 0 on success
   - `process.exit(1)` only on actual errors
3. Add integration tests that verify exit codes match success/failure states

## Related Files

Based on fspec architecture, likely locations:
- Command handlers for Event Storm operations
- Possibly shared error handling in base command infrastructure
