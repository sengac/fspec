# TUI-131 AST research: render sites that paint long text on one row

Run 2026-11-07 against the working tree (HEAD 38ce1c63b).

## `paint_left_aligned($$$)`
```
rust/fspec-tui/src/components/dialog_theme_paint.rs:78
rust/fspec-tui/src/components/dialog_theme.rs:201
rust/fspec-tui/src/components/dialog_theme.rs:288
rust/fspec-tui/src/components/dialog_theme.rs:290
rust/fspec-tui/src/views/agent/footer.rs:59
rust/fspec-tui/src/views/agent/footer.rs:65
rust/fspec-tui/src/views/agent/footer.rs:68
```

## `Paragraph::new($$$).render($$$)`
```
rust/fspec-tui/src/app/copy_notice.rs:94
rust/fspec-tui/src/components/hello.rs:72
rust/fspec-tui/src/components/list_scrollbar.rs:45
rust/fspec-tui/src/components/spinner.rs:71
rust/fspec-tui/src/views/agent/busy_composer.rs:95
rust/fspec-tui/src/views/agent/footer.rs:122
rust/fspec-tui/src/views/agent/footer.rs:145
rust/fspec-tui/src/views/agent/header.rs:131
rust/fspec-tui/src/views/agent/header.rs:143
rust/fspec-tui/src/views/agent/hitl_prompt.rs:144
rust/fspec-tui/src/views/agent/input_transition.rs:271
rust/fspec-tui/src/views/agent/mode_view_render.rs:190
rust/fspec-tui/src/views/agent/mode_view_render.rs:232
rust/fspec-tui/src/views/agent/mode_view_render.rs:32
rust/fspec-tui/src/views/agent/mode_view_render.rs:59
rust/fspec-tui/src/views/agent/mode_view_render.rs:81
rust/fspec-tui/src/views/agent/mode_view_render.rs:85
rust/fspec-tui/src/views/agent/multiline_input_render.rs:117
rust/fspec-tui/src/views/agent/multiline_input_render.rs:142
rust/fspec-tui/src/views/agent/multiline_input_render.rs:161
rust/fspec-tui/src/views/agent/multiline_input_render.rs:176
rust/fspec-tui/src/views/agent/multiline_input_render.rs:199
rust/fspec-tui/src/views/agent/multiline_input_render.rs:218
rust/fspec-tui/src/views/agent/pause_prompt.rs:140
rust/fspec-tui/src/views/agent/reflow_cache.rs:275
rust/fspec-tui/src/views/agent/role_banner.rs:43
rust/fspec-tui/src/views/agent/search_history_view_render.rs:35
rust/fspec-tui/src/views/agent/search_history_view_render.rs:91
rust/fspec-tui/src/views/agent/stdin_answer_box.rs:109
rust/fspec-tui/src/views/agent/stdin_answer_box.rs:150
rust/fspec-tui/src/views/agent/stdin_answer_box.rs:175
rust/fspec-tui/src/views/agent/stdin_answer_box.rs:194
rust/fspec-tui/src/views/blocklist/panes.rs:127
rust/fspec-tui/src/views/blocklist/panes.rs:136
rust/fspec-tui/src/views/blocklist/panes.rs:228
rust/fspec-tui/src/views/blocklist/panes.rs:256
rust/fspec-tui/src/views/board/borders.rs:15
rust/fspec-tui/src/views/board/checkpoint_status.rs:27
rust/fspec-tui/src/views/board/details_strip.rs:107
rust/fspec-tui/src/views/board/details_strip.rs:132
rust/fspec-tui/src/views/board/details_strip.rs:46
rust/fspec-tui/src/views/board/details_strip.rs:70
rust/fspec-tui/src/views/board/details_strip.rs:80
rust/fspec-tui/src/views/board/details_strip.rs:94
rust/fspec-tui/src/views/board/footer.rs:51
rust/fspec-tui/src/views/board/logo.rs:71
rust/fspec-tui/src/views/changed_files/render.rs:109
rust/fspec-tui/src/views/changed_files/render.rs:145
rust/fspec-tui/src/views/changed_files/render.rs:186
rust/fspec-tui/src/views/checkpoints/render.rs:132
rust/fspec-tui/src/views/checkpoints/render.rs:166
rust/fspec-tui/src/views/checkpoints/render.rs:231
rust/fspec-tui/src/views/checkpoints/render.rs:272
rust/fspec-tui/src/views/diff_common/pane.rs:48
rust/fspec-tui/src/views/diff_common/pane.rs:66
rust/fspec-tui/src/views/diff_common/pane.rs:86
rust/fspec-tui/src/views/full_screen_shell.rs:156
rust/fspec-tui/src/views/full_screen_shell.rs:198
rust/fspec-tui/src/views/full_screen_shell/title_closure_tests.rs:32
rust/fspec-tui/src/views/full_screen_shell/title_closure_tests.rs:36
```

## `$B.set_stringn($$$)`
```
rust/fspec-tui/src/components/board_card/frame.rs:56
rust/fspec-tui/src/components/board_card/frame.rs:58
rust/fspec-tui/src/components/board_card/page.rs:211
rust/fspec-tui/src/views/board/daemon_switcher.rs:165
rust/fspec-tui/src/views/board/daemon_switcher.rs:87
rust/fspec-tui/src/views/board/workspace_dropdown.rs:113
rust/fspec-tui/src/views/board/workspace_dropdown.rs:132
rust/fspec-tui/src/views/board/workspace_dropdown.rs:55
rust/fspec-tui/src/views/provider_settings/list_nav_render.rs:215
rust/fspec-tui/src/views/provider_settings/row_render.rs:145
rust/fspec-tui/src/views/provider_settings/row_segments.rs:115
rust/fspec-tui/src/views/provider_settings/row_segments.rs:124
```

## `render_footer_hint($$$)`
```
rust/fspec-tui/src/views/full_screen_shell.rs:117
rust/fspec-tui/src/views/full_screen_shell.rs:158
```

## `truncate_path($$$)`
```
rust/fspec-tui/src/views/diff_common/row.rs:63
```

## `wrapped_lines($$$)`
```
rust/fspec-tui/src/views/provider_settings/body_render.rs:123
rust/fspec-tui/src/views/provider_settings/oauth_login_render/copilot_preamble.rs:65
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:125
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:190
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:233
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:73
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:79
rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs:92
```

## `wrap_to_width($$$)`
```
rust/fspec-tui/src/components/dialog_theme_rows.rs:45
rust/fspec-tui/src/components/session_storage_dialog.rs:106
rust/fspec-tui/src/store/agent_view/diff_decode.rs:183
rust/fspec-tui/src/store/agent_view/diff_decode.rs:213
rust/fspec-tui/src/store/agent_view/diff_decode.rs:246
rust/fspec-tui/src/store/agent_view/diff_decode.rs:84
rust/fspec-tui/src/store/agent_view/diff_decode.rs:90
rust/fspec-tui/src/store/agent_view/markdown_tables_tests.rs:202
rust/fspec-tui/src/views/agent/text_wrap.rs:194
rust/fspec-tui/src/views/agent/text_wrap.rs:207
rust/fspec-tui/src/views/agent/text_wrap.rs:219
rust/fspec-tui/src/views/agent/text_wrap.rs:232
rust/fspec-tui/src/views/agent/text_wrap.rs:238
rust/fspec-tui/src/views/agent/text_wrap.rs:248
rust/fspec-tui/src/views/agent/text_wrap.rs:269
rust/fspec-tui/src/views/agent/text_wrap.rs:279
```

## Findings
- Every dialog row goes through dialog_theme_rows::paint_left_aligned (one cell per char, stops at the rect edge), so wrapping belongs in a row builder before render_dialog, not in the painter.
- Full-screen footers are one Paragraph row from mode_view_render::render_footer_hint inside full_screen_shell's fixed Length(1) footer (CHROME_ROWS = 3).
- board_card/frame.rs paints its status row and FOOTER with set_stringn at the inner width.
- diff_common::truncate_path keeps the head; it is called by changed_files and checkpoints rows.
- text_wrap::wrapped_lines / wrap_to_width (TUI-126) are already used by Provider Settings and the session storage dialog; they are the helpers to reuse.
- Per-site detail (file:line, strings, widths): /tmp/asst/prep/tui-dead-actions-copy-wrap-audit.md WR-1..WR-12, re-checked against HEAD.
