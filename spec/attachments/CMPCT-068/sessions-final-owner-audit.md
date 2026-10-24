# Sessions/UI/RPC final supervised pass

Preserves retained changes; no board changes, staging, commits, or gate edits.

## Current behavioral results

- `/tmp/cmpct-sessions-final-tests.log`: all fourteen explicit owned sessions
  targets GREEN, including the four genuine fresh-child restore atomicity
  regressions. Preflight runs before boundary/counter/writer installation.
- `/tmp/cmpct-tui-final-tests.log`: coherent fill3, header1 and committed
  projection4 GREEN. Actual runtime fill remains behavioral RED at line153:
  current physical occupancy8000 is correct, but displayed spend1700/300 is not
  the required cumulative3400/600 across the two physical segments.
  CLI owns the aggregation fix; sessions will rerun the unchanged acceptance.
- The retained `profile_model_limits::apply_direct_model` now passes actual
  Codex registry context/output limits into direct model selection. Missing
  registry values remain missing; no guessed limit or fixture override exists.
  Previous owner log `/tmp/cmpct-sessions-owner5-codex-repair.log` shows the
  actual cold child passes positive-limit restore and reaches the later
  gate-owned structural tool assertion. Gate owner owns full rerun evidence.

## Next checks

Strengthen independent audit's real-session serializer evidence and backend
ordering/unknown provenance coverage. Refresh incomplete/duplicate sidecar
ranges to exact test functions with one mapping per scenario/example row.
Fresh RPC, format, check, Clippy and specification validation follow source
stabilization. This is not a CMPCT-067 completion claim.
