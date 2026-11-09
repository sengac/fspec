# TUI-115 interview: running input and stdin prompts (decisions, 2026-11-01)

Decided with the user in an interactive interview, building on `fspec-running-input-ux-research.md`. Implemented by TUI-115, TUI-116 and TUI-117.

## TUI-115: running composer, steer and queue
- During **Running** and **Compacting** the composer stays visible under the activity line (spinner and status above it). A draft is never edited blind.
- **Running:** Enter steers (joins the turn at its next safe boundary) and Alt+Enter queues (runs after the turn). A hint line under the composer reads `Enter steer · Alt+Enter queue · Esc interrupt`.
- Submissions while busy are shown tagged `You (steer): …` or `Queued #n: …`, and queued inputs stay listed until they start.
- **Compacting:** the composer is editable, both Enter and Alt+Enter queue (steering into compaction makes no sense), and the hint says so.

## TUI-116: non-interrupting stdin notice and answer box
- A quiet-command stdin request (fspec's 3-second silence guess) shows a one-line notice above the composer: `<command> may be waiting for input · Ctrl+O answer · Ctrl+X dismiss`. It does not take focus or touch the chat draft.
- **Ctrl+O** opens a separate stdin answer box with its own buffer. The chat draft can never be written to the command, and it is exactly as left when focus returns. Ctrl+I was rejected because terminals send it as Tab, which already switches turns.
- **Stdin box focused:** Enter writes the line to the command's stdin. Esc returns focus to the chat draft and keeps the notice. Ctrl+C sends SIGINT to the command only. Ctrl+D sends EOF.
- **Chat box focused:** Esc still interrupts the agent turn. Ctrl+X dismisses the notice.

## TUI-117: recovery and per-session state
Decided after comparing VTCode (`/tmp/VTCode`). VTCode never guesses that a command wants input; the model writes stdin through a tool, or the user focuses a background session. On a failed write it restores the draft and shows a red error, and it clears its stdin target when the process ends. It keeps one global focus.
- **Failed write:** the text stays in the stdin box and a red inline error gives the reason. It is never cleared and only logged.
- **Exit or new output:** when the command exits or produces output again, the notice and stdin box close automatically. Unsent stdin text stays in the box's history (Shift+Up).
- **Dismiss:** Ctrl+X mutes further notices for that same command run, so a slow build doesn't nag.
- **Per session:** notices, the stdin draft and the chat draft belong to their session. Switching sessions shows that session's own state and never carries a draft or stdin target across, unlike VTCode's single global focus.

## Out of scope (user decision)
A model-driven stdin tool like VTCode's `write_stdin`. If wanted later, it gets a separate card.
