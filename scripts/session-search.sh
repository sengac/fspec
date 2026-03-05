#!/usr/bin/env bash
# session-search.sh — Search and reconstruct conversation context from fspec sessions
#
# Usage:
#   ./scripts/session-search.sh search "RLM-001"           # Search history for keyword
#   ./scripts/session-search.sh search "DeepSearch" --last 50  # Search last N entries
#   ./scripts/session-search.sh recent                      # Show recent sessions with summaries
#   ./scripts/session-search.sh recent --count 5            # Show N recent sessions
#   ./scripts/session-search.sh show <session-id>           # Reconstruct full conversation
#   ./scripts/session-search.sh show <session-id> --user-only  # Just user messages
#   ./scripts/session-search.sh context <keyword>           # Find sessions + show relevant turns
#
# Data model:
#   ~/.fspec/sessions/*.json    — session metadata + message_id references
#   ~/.fspec/messages/messages.jsonl — message records (content or blob refs)
#   ~/.fspec/blobs/XX/HASH      — large content stored by SHA-256
#   ~/.fspec/history.jsonl       — user input display log with timestamps
#
# Message content format (from persist_assistant_message_internal):
#   Assistant messages are stored as joined streaming chunks:
#     [Thinking: <first 50 chars of thinking chunk>...]   — thinking/reasoning
#     [Tool: <tool_name>]                                  — tool invocation
#     raw text fragments (split mid-word by SSE streaming) — response text
#   These are joined with \n. The script reassembles them into readable prose.

set -euo pipefail

FSPEC_DIR="${FSPEC_HOME:-$HOME/.fspec}"
SESSIONS_DIR="$FSPEC_DIR/sessions"
MESSAGES_FILE="$FSPEC_DIR/messages/messages.jsonl"
HISTORY_FILE="$FSPEC_DIR/history.jsonl"
BLOBS_DIR="$FSPEC_DIR/blobs"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
DIM='\033[2m'
BOLD='\033[1m'
RESET='\033[0m'

# Shared Python module for reassembling streamed assistant content.
#
# The persistence layer (session_manager.rs:4757-4767) stores each streaming
# event as a line:
#   - Thinking chunks: "[Thinking: <up to 50 chars>...]"
#   - Tool uses:       "[Tool: <name>]"
#   - Text chunks:     raw fragment strings (often split mid-word)
# All joined with "\n".
#
# This module concatenates adjacent same-type chunks back into readable blocks.
REASSEMBLE_PY='
import re

_TOOL_RE = re.compile(r"^\[Tool:\s*(.+?)\]$")
_TOOL_USE_RE = re.compile(r"^\[tool_use:\s*(.+?)\]$")
_TOOL_RESULT_RE = re.compile(r"^\[tool_result:\s*(.*)\]$")
# Matches [Thinking: ...] with closing bracket and optional trailing dots
_THINKING_CLOSED_RE = re.compile(r"\[Thinking:\s?(.*?)(?:\.\.\.?)?\]")
# Matches [Thinking: ... without closing bracket (truncated by persistence)
_THINKING_OPEN_RE = re.compile(r"\[Thinking:\s?(.*)")

def _tokenize_line(line):
    """Split a line into individual tokens, handling multiple [Thinking:] on one line.

    The persistence layer (session_manager.rs:4761-4764) truncates each thinking
    chunk to 50 chars and wraps it as:
        format!("[Thinking: {truncated}...]")
    However many chunks are stored WITHOUT a closing bracket — the truncation
    sometimes cuts the text before the "...]" suffix gets appended, or the
    chunk itself contains brackets that confuse the format. So we must handle
    both closed ([Thinking: text...]) and open ([Thinking: text) forms.

    Multiple tokens can appear on a single line without separators.
    """
    tokens = []
    pos = 0
    while pos < len(line):
        # Try closed form first: [Thinking: text...]
        m = _THINKING_CLOSED_RE.match(line, pos)
        if m:
            prefix = line[pos:m.start()]
            if prefix.strip():
                tokens.append(("text", prefix))
            tokens.append(("thinking", m.group(1)))
            pos = m.end()
            continue

        # Try open form: [Thinking: text (no closing bracket — rest of line)
        m_open = _THINKING_OPEN_RE.match(line, pos)
        if m_open:
            prefix = line[pos:m_open.start()]
            if prefix.strip():
                tokens.append(("text", prefix))
            tokens.append(("thinking", m_open.group(1)))
            pos = m_open.end()
            continue

        # Try [Tool: ...] at start of remaining
        remainder = line[pos:]
        m2 = _TOOL_RE.match(remainder) or _TOOL_USE_RE.match(remainder)
        if m2:
            prefix = line[pos:pos + m2.start()]
            if prefix.strip():
                tokens.append(("text", prefix))
            tokens.append(("tool", m2.group(1)))
            pos += m2.end()
            continue

        m3 = _TOOL_RESULT_RE.match(remainder)
        if m3:
            pos += m3.end()
            continue

        # No special token — advance to next potential token start
        next_bracket = line.find("[", pos + 1)
        if next_bracket == -1:
            tokens.append(("text", line[pos:]))
            break
        tokens.append(("text", line[pos:next_bracket]))
        pos = next_bracket

    return tokens


def reassemble_content(raw):
    """Reassemble stored streaming chunks into (type, content) sections.

    Returns a list of ("thinking", text), ("tool", name), ("text", text) tuples.

    The persistence layer (session_manager.rs persist_assistant_message_internal)
    stores each streaming event as:
      - Thinking: "[Thinking: <up to 50 chars>...]" — truncated at char boundary
        Each chunk is a fragment of the thinking text, often split mid-word.
        Leading whitespace is significant (word boundaries).
      - Tool use: "[Tool: <name>]"
      - Text: raw fragment string, also split mid-word by SSE streaming

    Multiple [Thinking:...] tokens can appear on a single line without any
    separator. We tokenize first, then merge adjacent same-type runs.
    """
    lines = raw.split("\n")
    sections = []
    buf_type = None   # "thinking" or "text"
    buf_parts = []

    def flush():
        nonlocal buf_type, buf_parts
        if buf_type and buf_parts:
            joined = "".join(buf_parts).strip()
            if joined:
                sections.append((buf_type, joined))
        buf_type = None
        buf_parts = []

    for line in lines:
        tokens = _tokenize_line(line)
        if not tokens:
            # Empty line — treat as text newline
            if buf_type == "text":
                buf_parts.append("\n")
            continue

        for ttype, tvalue in tokens:
            if ttype == "tool":
                flush()
                sections.append(("tool", tvalue))
            elif ttype == "thinking":
                if buf_type != "thinking":
                    flush()
                    buf_type = "thinking"
                buf_parts.append(tvalue)
            else:  # text
                if buf_type != "text":
                    flush()
                    buf_type = "text"
                buf_parts.append(tvalue)

    flush()
    return sections


def format_sections(sections, max_thinking=500):
    """Render reassembled sections as readable coloured output."""
    parts = []
    for stype, content in sections:
        if stype == "thinking":
            display = content
            if len(display) > max_thinking:
                display = display[:max_thinking] + "..."
            parts.append(f"\033[2m[Thinking] {display}\033[0m")
        elif stype == "tool":
            parts.append(f"\033[33m[Tool: {content}]\033[0m")
        elif stype == "text":
            parts.append(content)
    return "\n".join(parts)


def format_sections_full(sections):
    """Format with full thinking content (no truncation)."""
    return format_sections(sections, max_thinking=50000)


def plain_text(sections):
    """Extract just the searchable plain text from sections."""
    return " ".join(c for _, c in sections)
'

cmd_search() {
    local keyword="$1"
    shift
    local last_n=200

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --last) last_n="$2"; shift 2 ;;
            *) shift ;;
        esac
    done

    echo -e "${BOLD}Searching history for: ${CYAN}$keyword${RESET}"
    echo

    tail -n "$last_n" "$HISTORY_FILE" | python3 -c "
import json, sys, re

keyword = sys.argv[1]
pattern = re.compile(keyword, re.IGNORECASE)
sessions = {}

for line in sys.stdin:
    try:
        data = json.loads(line)
    except:
        continue
    display = data.get('display', '')
    if pattern.search(display):
        sid = data.get('session_id', '?')
        ts = data.get('timestamp', '?')[:19].replace('T', ' ')
        if sid not in sessions:
            sessions[sid] = []
        sessions[sid].append((ts, display))

if not sessions:
    print('No matches found.')
    sys.exit(0)

for sid, entries in sessions.items():
    print(f'\033[1m\033[34mSession: {sid}\033[0m')
    for ts, display in entries:
        preview = display[:200]
        if len(display) > 200:
            preview += '...'
        print(f'  \033[2m{ts}\033[0m  {preview}')
    print()
" "$keyword"
}

cmd_recent() {
    local count=10

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --count) count="$2"; shift 2 ;;
            *) shift ;;
        esac
    done

    echo -e "${BOLD}Recent sessions (last $count):${RESET}"
    echo

    python3 -c "
import json, os, sys
from pathlib import Path

sessions_dir = sys.argv[1]
history_file = sys.argv[2]
count = int(sys.argv[3])

session_files = sorted(
    Path(sessions_dir).glob('*.json'),
    key=lambda p: p.stat().st_mtime,
    reverse=True
)[:count]

session_msgs = {}
try:
    with open(history_file) as f:
        for line in f:
            try:
                data = json.loads(line)
                sid = data.get('session_id', '')
                display = data.get('display', '')
                ts = data.get('timestamp', '')
                if sid and display:
                    if sid not in session_msgs:
                        session_msgs[sid] = []
                    session_msgs[sid].append((ts, display))
            except:
                continue
except:
    pass

for sf in session_files:
    try:
        with open(sf) as f:
            data = json.load(f)
    except:
        continue

    sid = data.get('id', sf.stem)
    name = data.get('name', '')
    project = data.get('project', '?')
    provider = data.get('provider', '?')
    updated = data.get('updated_at', '?')[:19].replace('T', ' ')
    msg_count = len(data.get('messages', []))
    compaction = data.get('compaction')
    compacted = 'yes' if (isinstance(compaction, dict) and compaction.get('compacted')) else 'no'

    print(f'\033[1m\033[34m{sid}\033[0m')
    if name:
        print(f'  \033[1mName:\033[0m {name}')
    print(f'  \033[2mUpdated: {updated}  |  Provider: {provider}  |  Messages: {msg_count}  |  Compacted: {compacted}\033[0m')
    print(f'  \033[2mProject: {project}\033[0m')

    msgs = session_msgs.get(sid, [])
    if msgs:
        first_msg = msgs[0][1][:150]
        if len(msgs[0][1]) > 150:
            first_msg += '...'
        print(f'  \033[32mFirst:\033[0m {first_msg}')
        if len(msgs) > 1:
            last_msg = msgs[-1][1][:150]
            if len(msgs[-1][1]) > 150:
                last_msg += '...'
            print(f'  \033[32mLast:\033[0m  {last_msg}')
        print(f'  \033[2m({len(msgs)} user inputs)\033[0m')
    print()
" "$SESSIONS_DIR" "$HISTORY_FILE" "$count"
}

cmd_show() {
    local session_id="$1"
    shift
    local user_only=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --user-only) user_only=true; shift ;;
            *) shift ;;
        esac
    done

    local session_file="$SESSIONS_DIR/${session_id}.json"
    if [[ ! -f "$session_file" ]]; then
        echo -e "${RED}Session not found: $session_id${RESET}"
        exit 1
    fi

    python3 -c "
import json, sys, os, re

${REASSEMBLE_PY}

session_file = sys.argv[1]
messages_file = sys.argv[2]
blobs_dir = sys.argv[3]
user_only = sys.argv[4] == 'true'

with open(session_file) as f:
    session = json.load(f)

sid = session.get('id', '?')
name = session.get('name', '')
project = session.get('project', '?')
provider = session.get('provider', '?')
updated = session.get('updated_at', '?')[:19].replace('T', ' ')

print(f'\033[1m\033[34mSession: {sid}\033[0m')
if name:
    print(f'Name: {name}')
print(f'Project: {project}  |  Provider: {provider}  |  Updated: {updated}')
print(f'Messages: {len(session.get(\"messages\", []))}')
print('=' * 80)
print()

msg_ids = set()
for m in session.get('messages', []):
    mid = m.get('message_id', '')
    if mid:
        msg_ids.add(mid)

if not msg_ids:
    print('(No messages found)')
    sys.exit(0)

def resolve_blob(ref):
    if isinstance(ref, str) and ref.startswith('blob:sha256:'):
        hash_val = ref[len('blob:sha256:'):]
        prefix = hash_val[:2]
        blob_path = os.path.join(blobs_dir, prefix, hash_val)
        if os.path.exists(blob_path):
            with open(blob_path) as f:
                return f.read()
        return f'[blob not found: {hash_val}]'
    return ref

def extract_raw_text(content):
    if isinstance(content, str):
        resolved = resolve_blob(content)
        if resolved.startswith('[') or resolved.startswith('{'):
            try:
                parsed = json.loads(resolved)
                return extract_raw_text(parsed)
            except:
                pass
        return resolved
    elif isinstance(content, list):
        parts = []
        for item in content:
            if isinstance(item, dict):
                t = item.get('type', '')
                if t == 'text':
                    text = item.get('text', '')
                    parts.append(resolve_blob(text) if isinstance(text, str) else str(text))
                elif t == 'tool_use':
                    parts.append(f'[Tool: {item.get(\"name\", \"?\")}]')
                elif t == 'tool_result':
                    content_inner = item.get('content', '')
                    text = extract_raw_text(content_inner)
                    parts.append(f'[tool_result: {text[:300]}]')
                else:
                    text = item.get('text', str(item))
                    parts.append(str(text)[:500])
            elif isinstance(item, str):
                parts.append(resolve_blob(item))
        return '\n'.join(parts)
    elif isinstance(content, dict):
        t = content.get('type', '')
        if t == 'text':
            return resolve_blob(content.get('text', ''))
        return json.dumps(content)[:500]
    return str(content)[:500]

msg_order = [m.get('message_id') for m in session.get('messages', [])]

msg_lookup = {}
with open(messages_file) as f:
    for line in f:
        try:
            data = json.loads(line)
            mid = data.get('id', '')
            if mid in msg_ids:
                msg_lookup[mid] = data
        except:
            continue

for mid in msg_order:
    if mid not in msg_lookup:
        continue
    msg = msg_lookup[mid]
    role = msg.get('role', '?')

    if user_only and role != 'user':
        continue

    content = msg.get('content', '')
    raw_text = extract_raw_text(content)

    color = '\033[32m' if role == 'user' else '\033[36m' if role == 'assistant' else '\033[33m'
    ts = msg.get('created_at', '')[:19].replace('T', ' ')

    print(f'{color}\033[1m[{role}]\033[0m \033[2m{ts}\033[0m')

    if role == 'assistant':
        sections = reassemble_content(raw_text)
        text = format_sections_full(sections)
    else:
        text = raw_text

    if len(text) > 5000:
        text = text[:5000] + f'\n... [{len(text) - 5000} chars truncated]'

    print(text)
    print()
" "$session_file" "$MESSAGES_FILE" "$BLOBS_DIR" "$user_only"
}

cmd_context() {
    local keyword="$1"
    shift
    local last_n=500
    local show_turns=5

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --last) last_n="$2"; shift 2 ;;
            --turns) show_turns="$2"; shift 2 ;;
            *) shift ;;
        esac
    done

    echo -e "${BOLD}Finding sessions with context about: ${CYAN}$keyword${RESET}"
    echo

    local matching_sessions
    matching_sessions=$(tail -n "$last_n" "$HISTORY_FILE" | python3 -c "
import json, sys, re
keyword = sys.argv[1]
pattern = re.compile(keyword, re.IGNORECASE)
sessions = set()
for line in sys.stdin:
    try:
        data = json.loads(line)
    except:
        continue
    display = data.get('display', '')
    if pattern.search(display):
        sessions.add(data.get('session_id', ''))
import os
sessions_dir = sys.argv[2]
for f in os.listdir(sessions_dir):
    if not f.endswith('.json'):
        continue
    try:
        with open(os.path.join(sessions_dir, f)) as fh:
            sdata = json.load(fh)
        name = sdata.get('name', '')
        if pattern.search(name):
            sessions.add(sdata.get('id', f[:-5]))
    except:
        continue
for s in sessions:
    if s:
        print(s)
" "$keyword" "$SESSIONS_DIR")

    if [[ -z "$matching_sessions" ]]; then
        echo -e "${YELLOW}No matches in history. Searching message content (slower)...${RESET}"
        matching_sessions=$(grep -i "$keyword" "$MESSAGES_FILE" 2>/dev/null | head -20 | python3 -c "
import json, sys
ids = set()
for line in sys.stdin:
    try:
        data = json.loads(line)
        ids.add(data.get('id', ''))
    except:
        continue
import os
sessions_dir = sys.argv[1]
for f in os.listdir(sessions_dir):
    if not f.endswith('.json'):
        continue
    try:
        with open(os.path.join(sessions_dir, f)) as fh:
            sdata = json.load(fh)
        msg_ids = {m.get('message_id') for m in sdata.get('messages', [])}
        if msg_ids & ids:
            print(sdata.get('id', f[:-5]))
    except:
        continue
" "$SESSIONS_DIR")
    fi

    if [[ -z "$matching_sessions" ]]; then
        echo -e "${RED}No sessions found mentioning '$keyword'${RESET}"
        exit 0
    fi

    echo "$matching_sessions" | while IFS= read -r sid; do
        [[ -z "$sid" ]] && continue
        local session_file="$SESSIONS_DIR/${sid}.json"
        [[ ! -f "$session_file" ]] && continue

        echo -e "${BOLD}${BLUE}━━━ Session: $sid ━━━${RESET}"

        python3 -c "
import json, sys, os, re

${REASSEMBLE_PY}

session_file = sys.argv[1]
messages_file = sys.argv[2]
blobs_dir = sys.argv[3]
keyword = sys.argv[4]
max_turns = int(sys.argv[5])

pattern = re.compile(keyword, re.IGNORECASE)

with open(session_file) as f:
    session = json.load(f)

name = session.get('name', '')
updated = session.get('updated_at', '?')[:19].replace('T', ' ')
provider = session.get('provider', '?')
msg_count = len(session.get('messages', []))

if name:
    print(f'  Name: {name}')
print(f'  Updated: {updated}  |  Provider: {provider}  |  Messages: {msg_count}')

msg_ids = set()
msg_order = []
for m in session.get('messages', []):
    mid = m.get('message_id', '')
    if mid:
        msg_ids.add(mid)
        msg_order.append(mid)

def resolve_blob(ref):
    if isinstance(ref, str) and ref.startswith('blob:sha256:'):
        hash_val = ref[len('blob:sha256:'):]
        prefix = hash_val[:2]
        blob_path = os.path.join(blobs_dir, prefix, hash_val)
        if os.path.exists(blob_path):
            with open(blob_path) as f:
                return f.read()
        return f'[blob not found]'
    return ref

def extract_raw_text(content):
    if isinstance(content, str):
        resolved = resolve_blob(content)
        if resolved.startswith('[') or resolved.startswith('{'):
            try:
                parsed = json.loads(resolved)
                return extract_raw_text(parsed)
            except:
                pass
        return resolved
    elif isinstance(content, list):
        parts = []
        for item in content:
            if isinstance(item, dict):
                t = item.get('type', '')
                if t == 'text':
                    text = item.get('text', '')
                    parts.append(resolve_blob(text) if isinstance(text, str) else str(text))
                elif t == 'tool_use':
                    parts.append(f'[Tool: {item.get(\"name\",\"?\")}]')
                elif t == 'tool_result':
                    parts.append('[tool_result]')
                else:
                    text = item.get('text', str(item))
                    parts.append(str(text)[:500])
            elif isinstance(item, str):
                parts.append(resolve_blob(item))
        return '\n'.join(parts)
    elif isinstance(content, dict):
        if content.get('type') == 'text':
            return resolve_blob(content.get('text', ''))
        return json.dumps(content)[:300]
    return str(content)[:300]

msg_lookup = {}
with open(messages_file) as f:
    for line in f:
        try:
            data = json.loads(line)
            mid = data.get('id', '')
            if mid in msg_ids:
                msg_lookup[mid] = data
        except:
            continue

# Build display text for each message, reassembling assistant chunks
matching_indices = []
texts_by_index = {}
for i, mid in enumerate(msg_order):
    if mid in msg_lookup:
        msg = msg_lookup[mid]
        raw_text = extract_raw_text(msg.get('content', ''))
        role = msg.get('role', '?')
        ts = msg.get('created_at', '')

        if role == 'assistant':
            sections = reassemble_content(raw_text)
            display_text = format_sections(sections)
            search_text = plain_text(sections)
        else:
            display_text = raw_text
            search_text = raw_text

        texts_by_index[i] = (role, display_text, ts)
        if pattern.search(search_text):
            matching_indices.append(i)

if not matching_indices:
    print(f'  (keyword not in message content, showing last {max_turns} turns)')
    start = max(0, len(msg_order) - max_turns)
    matching_indices = list(range(start, len(msg_order)))

shown = set()
print()
for idx in matching_indices[-max_turns:]:
    for i in range(max(0, idx - 1), min(len(msg_order), idx + 2)):
        if i in shown or i not in texts_by_index:
            continue
        shown.add(i)
        role, text, ts = texts_by_index[i]
        ts_short = ts[:19].replace('T', ' ') if ts else ''
        color = '\033[32m' if role == 'user' else '\033[36m' if role == 'assistant' else '\033[33m'

        max_len = 3000 if i in matching_indices else 500
        if len(text) > max_len:
            text = text[:max_len] + f'... [{len(text) - max_len} chars truncated]'

        print(f'  {color}\033[1m[{role}]\033[0m \033[2m{ts_short}\033[0m')
        for line in text.split('\n'):
            print(f'    {line}')
        print()
    if idx < matching_indices[-1]:
        print(f'  \033[2m--- ... ---\033[0m')
        print()
" "$session_file" "$MESSAGES_FILE" "$BLOBS_DIR" "$keyword" "$show_turns"

        echo
    done
}

# Main dispatch
case "${1:-help}" in
    search)
        shift
        cmd_search "$@"
        ;;
    recent)
        shift
        cmd_recent "$@"
        ;;
    show)
        shift
        cmd_show "$@"
        ;;
    context)
        shift
        cmd_context "$@"
        ;;
    help|--help|-h)
        echo "Usage: session-search.sh <command> [args]"
        echo
        echo "Commands:"
        echo "  search <keyword> [--last N]     Search user input history for keyword"
        echo "  recent [--count N]              Show N most recent sessions with summaries"
        echo "  show <session-id> [--user-only] Reconstruct full conversation"
        echo "  context <keyword> [--last N] [--turns N]"
        echo "                                  Find sessions + show relevant turns"
        echo
        echo "Examples:"
        echo "  ./scripts/session-search.sh search 'RLM-001'"
        echo "  ./scripts/session-search.sh recent --count 5"
        echo "  ./scripts/session-search.sh show abc-123-def"
        echo "  ./scripts/session-search.sh context 'DeepSearch' --turns 10"
        ;;
    *)
        echo "Unknown command: $1 (try --help)"
        exit 1
        ;;
esac