# BUG-350 root cause

TUI-114 (4b979c82) added a third Zone C button, `[ New Bot ]`, in front of `[ New Agent ]` and `[ Close Agent ]` (`AGENT_ZONE_C` in `rust/fspec-tui/src/views/agent/menu_render.rs:49`). It updated the MENU-007 feature, whose scenarios now say "Right six times … 'New Bot' then 'New Agent' then 'Close Agent'". It did not update `rust/fspec-tui/tests/menu007_agent_board_view_item.rs`, which still walked a 2-button Zone C: five Rights, with `ZoneC(1)` as the last stop.
The ring code (`walk_menu_ring`, `views/agent/menu_keys.rs`) is correct. With 2 chips and 3 buttons, the 5th Right lands on `ZoneC(2)` 'Close Agent', and Left from 'Board View' wraps to `ZoneC(2)`. The tests and their `@step` comments were stale, and link-coverage rejected them.

Fix: the ring test walks the five stops (chips #1 and #2, then 'New Bot', 'New Agent' and 'Close Agent') and wraps on the 6th Right. The left-wrap test expects `ZoneC(2)`. Both now carry the feature's exact step text.
