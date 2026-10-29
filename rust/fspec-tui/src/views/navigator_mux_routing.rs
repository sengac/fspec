impl Navigator {
    /// MUX-001: route an event through the mux grid. Keyboard input
    /// goes to the focused pane ONLY (the isolation "trap"); mouse
    /// events hit-test the divider (drag) then the pane rects
    /// (click-to-focus + forward).
    pub(crate) fn handle_mux_event(
        &mut self,
        event: &Event,
        board_store: &BoardStore,
    ) -> EventResult {
        let is_mouse = matches!(event, Event::Mouse(_));
        if is_mouse {
            // MENU-004 R-MOUSE: the top-row bar hit-tests run BEFORE the
            // dividers/panes (the bar is an overlay control — R-MOUSE:
            // "a bar click never changes pane focus"). `Pass` falls
            // through to the usual divider/pane routing below.
            match menu_mouse::classify_bar_mouse(&self.mux, event) {
                menu_mouse::MuxBarMouseDecision::Pass => {}
                menu_mouse::MuxBarMouseDecision::Claimed(action) => {
                    self.emit_menu_action(*action);
                    return EventResult::consumed();
                }
                menu_mouse::MuxBarMouseDecision::Swallowed => {
                    return EventResult::consumed();
                }
                menu_mouse::MuxBarMouseDecision::Ignore => {
                    return EventResult::ignored();
                }
                // A left click outside the open dropdown closes it AND
                // clears the bar's ring focus (BUG-196 R3: the
                // `MenuDismissBar` 'close + de-select' gesture — the
                // board store / mux layout each drop their focus) AND
                // the click still lands: the divider/pane routing below
                // runs on the same event (R-MOUSE: "the click still
                // lands on the pane").
                menu_mouse::MuxBarMouseDecision::CloseOutsideThenLand => {
                    self.emit_menu_action(Action::MenuDismissBar);
                }
            }
            let decision = mux_mouse::classify_mouse(&self.mux, event);
            return match decision {
                mux_mouse::MouseDecision::DividerDown { index } => {
                    self.mux.begin_drag(index);
                    EventResult::consumed()
                }
                mux_mouse::MouseDecision::DividerDrag { index } => {
                    if self.mux.is_dragging {
                        if let Some((col, row)) = mux_mouse::mouse_pos(event) {
                            let (pos, total) = self.mux_drag_axis(col, row, index);
                            let horizontal =
                                self.mux.config().orientation == MuxOrientation::Horizontal;
                            let cursor = if horizontal { col } else { row };
                            // BUG-166: live width = cursor minus the
                            // DRAGGED pane's origin (the drag tracks the
                            // cursor); the stored percent is relative to
                            // that pane's width over the available axis,
                            // so release keeps the divider in place.
                            let pane_start = self
                                .mux
                                .pane_rects()
                                .get(index)
                                .map(|r| if horizontal { r.x } else { r.y })
                                .unwrap_or(0);
                            let width = cursor.saturating_sub(pane_start);
                            self.mux.update_drag(index, width, pos, total);
                        }
                    }
                    EventResult::consumed()
                }
                mux_mouse::MouseDecision::DividerUp { .. } => {
                    self.mux.finish_drag();
                    EventResult::consumed()
                }
                mux_mouse::MouseDecision::Pane { index } => {
                    self.mux.set_focus(index);
                    let result = self.forward_mux_event_to_focused_pane(event, board_store);
                    if result.is_consumed() {
                        result
                    } else {
                        EventResult::consumed()
                    }
                }
                mux_mouse::MouseDecision::Gap => EventResult::ignored(),
            };
        }
        // BUG-179: a bracketed paste (Event::Paste) — including a terminal
        // file drop delivered as a paste — honours the same keyboard
        // isolation as keys: it reaches the FOCUSED pane only. The focused
        // pane's handler decides: the Agent pane's paste precedence/gate
        // chain (HITL / exec-stdin / compacting / CRLF) inserts into the
        // live composer; the Board/Files/Checkpoints panes ignore it.
        // Pre-fix the Event::Key guard below dropped every non-key event
        // here, so the paste was silently lost.
        if matches!(event, Event::Paste(_)) {
            return self.forward_mux_event_to_focused_pane(event, board_store);
        }
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };
        let key = *key;
        // MENU-004 R-KEYS: the top-row bar claims its keys BEFORE the
        // R8 board-Enter interceptor and before the focused pane sees
        // them (an open dropdown is true-modal; a focused bar swallows
        // unknown keys; the entry rules land on Item(0) via
        // `MenuMoveToItem(0)`).
        match menu_keys::classify_bar_key(&self.mux, board_store, &self.agent, &key) {
            menu_keys::MuxBarKeyOutcome::Emit(action) => {
                self.emit_menu_action(*action);
                return EventResult::consumed();
            }
            menu_keys::MuxBarKeyOutcome::Swallow => {
                return EventResult::consumed();
            }
            menu_keys::MuxBarKeyOutcome::DismissAndForward => {
                // A bare character with the bar focused: clear the bar
                // focus AND type the SAME key into the focused pane's
                // input on this keystroke (R-KEYS).
                self.mux.menu_dismiss();
                return self.forward_mux_event_to_focused_pane(&Event::Key(key), board_store);
            }
            menu_keys::MuxBarKeyOutcome::Forward => {}
        }
        // R8: Enter on the focused BOARD pane in mux mode binds the
        // selected work unit + focuses the agent pane WITHOUT flipping
        // the whole view. Intercepted here (before the board handler
        // would emit EnterWorkUnit).
        if key.code == crossterm::event::KeyCode::Enter
            && self.mux.focus() < self.mux.effective_panes().len()
            && self.mux.effective_panes()[self.mux.focus()]
                == crate::views::multiplex::MuxPaneKind::Board
        {
            if let Some(unit) = board_store.selected_work_unit() {
                if let Some(tx) = &self.action_tx {
                    let _ = tx.send(Action::MuxEnterWorkUnit(unit.id.clone()));
                }
                return EventResult::consumed();
            }
        }
        let decision = mux_keys::classify_key(&self.mux, &key);
        match decision {
            mux_keys::KeyDecision::FocusPrev => {
                // MUX-002: agent window backward-rotation OR focus
                // movement (stops at the first pane — no wrap).
                self.mux.shift_left();
                EventResult::consumed()
            }
            mux_keys::KeyDecision::FocusNext => {
                // MUX-002: agent window forward-rotation, focus
                // movement, or a new-agent prompt at the right edge.
                if self.mux.shift_right() {
                    self.emit_mux_new_agent();
                }
                EventResult::consumed()
            }
            mux_keys::KeyDecision::Forward => {
                // Forward to the focused pane; if the pane ignores the
                // key, fall through to the App-level shortcuts (e.g.
                // '?' help, 'm' mux toggle) — mirroring the
                // single-view cascade.
                self.forward_mux_event_to_focused_pane(&Event::Key(key), board_store)
            }
        }
    }
}
