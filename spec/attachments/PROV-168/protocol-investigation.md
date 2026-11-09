# Current Anthropic OAuth protocol investigation

Investigated the installed Claude Code `2.1.316`, rather than assuming that changing
the old version string alone proves compatibility. Both original markdown
attachments informed this investigation.

## Reproducible, credential-free capture

Started a loopback HTTP listener and launched the installed Claude executable with
an isolated temporary configuration, a dummy subscription token,
`ANTHROPIC_BASE_URL` pointing to the listener, and:

```text
claude -p 'what is 3 + 2' --model claude-opus-5-5 --tools '' --no-session-persistence
```

No real credentials were sent to the listener. The listener deliberately returned
HTTP 400, so this capture demonstrates the request shape, not server acceptance.
The CLI made additional compatibility attempts; fspec must not reproduce those
silent capability fallbacks.

## Observed request

| Component | Claude Code 2.1.316 capture | fspec decision |
| --- | --- | --- |
| Endpoint | `/v1/messages?beta=true` | Preserve |
| Authentication | Bearer OAuth | Preserve; never send an API key alongside it |
| User agent | `claude-cli/2.1.316 (external, sdk-cli)` | Resolve the release dynamically; fspec uses its CLI entrypoint |
| Application | `x-app: cli` | Preserve |
| Messages version | `anthropic-version: 2023-06-01` | Preserve |
| OAuth beta | `oauth-2025-04-20` | Preserve |
| Claude Code beta | `claude-code-20250219` | Preserve |
| Thinking | `type: adaptive`, `display: omitted` | Preserve adaptive; request `summarized` for fspec's visible thinking |
| Effort | `output_config.effort: medium` | Map the user's low/medium/high selection |
| Maximum output | `max_tokens: 128000` | Continue using fspec's resolved output reservation, not the SDK's default |
| System identity | Claude Agent SDK prefix for this SDK-mode invocation | Keep the existing supported Claude Code CLI prefix |
| Metadata | Device, account, session identifiers | Do not fabricate SDK account or device telemetry |
| Billing attribution | `x-anthropic-billing-header` system block | Do not fabricate Claude Code billing attribution |

Additional SDK betas observed: interleaved thinking, context management, per-turn
control, mid-conversation tool changes, thinking token counts, effort, and
extended cache TTL. Their presence is not evidence that they are required for
fspec's request. Do not advertise context editing, 1h caching, per-turn changes,
or SDK-specific counters without implementing them. Current adaptive thinking
interleaves automatically; legacy models retain their interleaved-thinking beta.
The already-supported prompt-caching beta remains compatible with fspec's
ephemeral cache markers.

## Version source and failure behavior

The installed `claude --version` is authoritative unless a deployment explicitly
sets `FSPEC_CLAUDE_CODE_VERSION`. The override is a strict release version, not a
complete user-agent string. Invalid overrides must not fall through to another
source. No compiled current-version fallback is permitted. The incident supplies
the minimum compatibility baseline, `2.1.280`; this is a lower-bound check, not a
claimed current release. Server-enforced newer minima remain authoritative and
must produce actionable update guidance, including the original request ID.

## Context display

Preflight resolves model capacity before inference. The initial context snapshot
deliberately has unknown occupancy until provider usage arrives. A pre-stream
HTTP rejection does not establish measured occupancy. The TUI's unknown-context
label therefore remains correct even with known model capacity. Fresh zero token
counters are separate from unknown context occupancy; no zero-percent fill or
successful usage should be manufactured.

## Official references

- https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking
- https://platform.claude.com/docs/en/build-with-claude/extended-thinking
- https://platform.claude.com/docs/en/build-with-claude/effort

The current documentation explicitly shows Opus 5.5 effort under
`output_config.effort`; adaptive interleaving needs no beta. Manual
`budget_tokens` thinking is unsupported on Claude 4.7 and later. Low, medium, and
high remain valid effort selections; fspec does not invent additional UI levels.

## Authorized verification

After implementation, the opt-in smoke check used the real stored fspec OAuth
credentials and the installed `2.1.316` identity against Anthropic, not the
loopback fixture. Both a non-streaming request and a subsequent streaming request
to `claude-opus-5-5` succeeded with adaptive thinking, summarized display, and
medium effort. The streaming request returned exactly `5` for `what is 3 + 2`,
with nonzero input/output counts and complete terminal usage. No SDK billing
attribution, account metadata, or additional SDK-only betas were needed for these
authorized requests. No credentials or token values are recorded in this report.
