//! Key/mouse event dispatch for the browse mode-view.
//!
//! Extracted from `mod.rs` (PROV-107) to keep that file under the
//! 300-LoC ceiling. Behaviour-preserving move of `impl ModelSelectorView`
//! methods; field/method visibility unchanged.

use super::*;

impl ModelSelectorView {
    pub fn handle_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        // RPC-344: custom-model form/confirm overlays intercept input BEFORE
        // the browse/filter handlers (TS handleDeleteConfirmInput +
        // handleCustomModelFormInput run first, ModelSelectorScreen.tsx:124-131).
        match &self.custom_model_mode {
            CustomModelMode::Add { .. } | CustomModelMode::Edit { .. } => {
                return self.handle_form_key(key);
            }
            CustomModelMode::DeleteConfirm { .. } => {
                return self.handle_delete_confirm_key(key);
            }
            CustomModelMode::Browse => {}
        }
        if self.filter_mode {
            return self.handle_filter_key(key);
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ModelSelectorEvent::Ignored;
        }
        match key.code {
            KeyCode::Esc => ModelSelectorEvent::Close,
            KeyCode::Char('/') => {
                self.filter_mode = true;
                // PROV-104 parity: TS resets the scroll offset to 0 when the
                // filter opens (useModelSelectorState.ts:288). Make the reset
                // explicit here rather than relying on a render-time
                // side-effect. The filter-change handlers (Esc/Backspace/Char)
                // already pair anchor_first_selectable() with adjust_scroll();
                // this completes parity for the open path. Observable behavior
                // is unchanged (offset already reconciles on the first render).
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.is_refreshing = true;
                ModelSelectorEvent::Emit(Action::RefreshModelSelector)
            }
            // RPC-345: Tab leaves the selector for Provider Settings,
            // completing the bidirectional Tab toggle. Filter mode is
            // handled earlier in handle_filter_key, so Tab while typing a
            // filter never reaches here (mirror of provider_settings/list.rs:62).
            KeyCode::Tab => ModelSelectorEvent::SwitchToProviders,
            // RPC-344: a/e/d open the custom-model CRUD overlays, gated on the
            // focused row. They are consumed no-ops on non-eligible rows.
            KeyCode::Char('a') => {
                self.try_open_add_form();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char('e') => {
                self.try_open_edit_form();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char('d') => {
                self.try_open_delete_confirm();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Up => {
                self.move_up();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Down => {
                self.move_down();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Home => {
                self.anchor_first_selectable();
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::PageDown => {
                self.page_down();
                ModelSelectorEvent::Consumed
            }
            KeyCode::PageUp => {
                self.page_up();
                ModelSelectorEvent::Consumed
            }
            KeyCode::End => {
                self.selected_index =
                    crate::components::model_selector_dialog_rows::last_selectable(&self.rows);
                self.has_selection = true;
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Left => {
                self.toggle_expansion(false);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Right => {
                self.toggle_expansion(true);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Enter => {
                // [MODEL-SELECT] DEBUG: trace the Enter decision path end-to-end.
                tracing::info!(
                    target: "model_select",
                    selected_index = self.selected_index,
                    row_count = self.rows.len(),
                    has_selection = self.has_selection,
                    session_id = ?self.session_id,
                    "[MODEL-SELECT] Enter pressed in browse mode"
                );
                // Copy the focused row's fields out FIRST so the immutable
                // borrow is dropped before any &mut self call below.
                let Some((selectable, provider_key, model_id)) = self
                    .rows
                    .get(self.selected_index)
                    .map(|r| (r.selectable, r.provider_key.clone(), r.model_id.clone()))
                else {
                    tracing::info!(
                        target: "model_select",
                        selected_index = self.selected_index,
                        "[MODEL-SELECT] Enter: no row at selected_index -> Consumed (no-op)"
                    );
                    return ModelSelectorEvent::Consumed;
                };
                tracing::info!(
                    target: "model_select",
                    selectable,
                    provider_key = %provider_key,
                    model_id = %model_id,
                    "[MODEL-SELECT] Enter: focused row resolved"
                );
                // TS parity (ModelSelectorScreen.tsx:203-210): Enter on a
                // non-selectable provider/profile header TOGGLES its expansion
                // (expand if collapsed, collapse if expanded) rather than being
                // a no-op — without this the user can never reach a model with
                // Enter alone on a fresh collapse-by-default open.
                if !selectable {
                    let expanded = self.is_expanded(&provider_key);
                    tracing::info!(
                        target: "model_select",
                        provider_key = %provider_key,
                        was_expanded = expanded,
                        "[MODEL-SELECT] Enter on header -> toggling expansion"
                    );
                    self.toggle_expansion(!expanded);
                    return ModelSelectorEvent::Consumed;
                }
                // PROV-101: never silently select when nothing is highlighted.
                // With no active selection (no current model, no explicit nav)
                // Enter is a consumed no-op rather than picking row 0.
                if !self.has_selection {
                    tracing::info!(
                        target: "model_select",
                        "[MODEL-SELECT] Enter on model row but has_selection=false -> Consumed (no-op, PROV-101)"
                    );
                    return ModelSelectorEvent::Consumed;
                }
                // PROV-117 / TS parity (ModelSelectorScreen.tsx:203-210 +
                // modelSelectionService.selectModel): the Enter handler has NO
                // session-existence guard. The selection is emitted regardless
                // of whether a session is active; App::dispatch gates only the
                // backend model write on session presence, and the selector
                // closes either way (Navigator::apply_action on ModelSelected).
                tracing::info!(
                    target: "model_select",
                    session_id = ?self.session_id,
                    provider_key = %provider_key,
                    model_id = %model_id,
                    "[MODEL-SELECT] Enter -> EMIT Action::ModelSelected"
                );
                ModelSelectorEvent::Emit(Action::ModelSelected(
                    self.session_id.clone(),
                    provider_key,
                    model_id,
                ))
            }
            _ => ModelSelectorEvent::Consumed,
        }
    }

    /// Route a mouse-wheel event: ScrollUp/ScrollDown advance the
    /// selection across selectable rows (skipping headers), mirroring
    /// the retired modal's wheel behaviour. RPC-353: the shared
    /// `WheelVelocity` 1×–5× ramp drives the step count so rapid wheel
    /// events move multiple rows per event (same feel as the chat view).
    pub fn handle_mouse(&mut self, ev: crossterm::event::MouseEvent) -> ModelSelectorEvent {
        use crate::components::scroll_viewport::WheelDirection;
        use crossterm::event::MouseEventKind;
        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return ModelSelectorEvent::Ignored,
        };
        let step = self.wheel.step(dir);
        let mover: fn(&mut Self) = match dir {
            WheelDirection::Up => Self::move_up,
            WheelDirection::Down => Self::move_down,
        };
        for _ in 0..step.unsigned_abs() {
            mover(self);
        }
        ModelSelectorEvent::Consumed
    }
}
