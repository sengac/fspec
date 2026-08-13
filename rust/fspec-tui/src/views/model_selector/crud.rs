//! RPC-344 custom-model CRUD: form/confirm open, submit & filter input.
//!
//! Extracted from `mod.rs` (PROV-107) to keep that file under the
//! 300-LoC ceiling. Behaviour-preserving move of `impl ModelSelectorView`
//! methods; field/method visibility unchanged.

use super::*;

impl ModelSelectorView {
    pub(crate) fn focused_row(
        &self,
    ) -> Option<&crate::components::model_selector_dialog_rows::ModelSelectorRow> {
        self.rows.get(self.selected_index)
    }

    /// `a`: open the Add form when the focused row belongs to a profile
    /// section (header OR model). A consumed no-op on non-profile rows.
    pub(crate) fn try_open_add_form(&mut self) {
        let Some(row) = self.focused_row() else {
            return;
        };
        let Some(profile_name) = row.profile_name.clone() else {
            return;
        };
        let provider_id = row.provider_key.clone();
        self.custom_model_mode = CustomModelMode::Add {
            provider_id,
            profile_name,
        };
        self.form = CustomModelForm::default();
    }

    /// `e`: open the Edit form when the focused row is a selectable custom
    /// model inside a profile section. A consumed no-op otherwise.
    pub(crate) fn try_open_edit_form(&mut self) {
        let Some(row) = self.focused_row() else {
            return;
        };
        if !row.selectable || !row.is_custom {
            return;
        }
        let Some(profile_name) = row.profile_name.clone() else {
            return;
        };
        let provider_id = row.provider_key.clone();
        let original_model_id = row.model_id.clone();
        self.form = CustomModelForm::prefill_from_entry(
            &row.model_id,
            &row.label,
            row.context_window,
            row.supports_reasoning,
            row.supports_vision,
        );
        self.custom_model_mode = CustomModelMode::Edit {
            provider_id,
            profile_name,
            original_model_id,
        };
    }

    /// `d`: open the delete confirmation when the focused row is a selectable
    /// custom model inside a profile section. A consumed no-op otherwise.
    pub(crate) fn try_open_delete_confirm(&mut self) {
        let Some(row) = self.focused_row() else {
            return;
        };
        if !row.selectable || !row.is_custom {
            return;
        }
        let Some(profile_name) = row.profile_name.clone() else {
            return;
        };
        self.custom_model_mode = CustomModelMode::DeleteConfirm {
            provider_id: row.provider_key.clone(),
            profile_name,
            model_id: row.model_id.clone(),
            display_name: row.label.clone(),
        };
    }

    /// Route a key through the open add/edit form, building the matching
    /// Action on a valid submit (returns the view to browse mode).
    pub(crate) fn handle_form_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        match self.form.handle_key(key) {
            FormOutcome::Editing => ModelSelectorEvent::Consumed,
            FormOutcome::Cancel => {
                self.custom_model_mode = CustomModelMode::Browse;
                self.form = CustomModelForm::default();
                ModelSelectorEvent::Consumed
            }
            FormOutcome::Submit => self.submit_form(),
        }
    }

    /// Build the definition and emit Add/Edit; an empty Model ID keeps the
    /// form open (no Action emitted).
    pub(crate) fn submit_form(&mut self) -> ModelSelectorEvent {
        let Some(definition) = self.form.build_definition() else {
            return ModelSelectorEvent::Consumed;
        };
        let action = match &self.custom_model_mode {
            CustomModelMode::Add {
                provider_id,
                profile_name,
            } => Action::AddCustomModel {
                provider_id: provider_id.clone(),
                profile_name: profile_name.clone(),
                definition,
            },
            CustomModelMode::Edit {
                provider_id,
                profile_name,
                original_model_id,
            } => Action::EditCustomModel {
                provider_id: provider_id.clone(),
                profile_name: profile_name.clone(),
                original_model_id: original_model_id.clone(),
                definition,
            },
            _ => return ModelSelectorEvent::Consumed,
        };
        self.custom_model_mode = CustomModelMode::Browse;
        self.form = CustomModelForm::default();
        ModelSelectorEvent::Emit(action)
    }

    /// Route a key through the delete-confirm overlay: y/Enter confirm,
    /// n/Esc cancel.
    pub(crate) fn handle_delete_confirm_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        let confirm = matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')
        );
        let cancel = matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N')
        );
        if confirm {
            let action = if let CustomModelMode::DeleteConfirm {
                provider_id,
                profile_name,
                model_id,
                ..
            } = &self.custom_model_mode
            {
                Some(Action::DeleteCustomModel {
                    provider_id: provider_id.clone(),
                    profile_name: profile_name.clone(),
                    model_id: model_id.clone(),
                })
            } else {
                None
            };
            self.custom_model_mode = CustomModelMode::Browse;
            return match action {
                Some(a) => ModelSelectorEvent::Emit(a),
                None => ModelSelectorEvent::Consumed,
            };
        }
        if cancel {
            self.custom_model_mode = CustomModelMode::Browse;
        }
        ModelSelectorEvent::Consumed
    }

    pub(crate) fn handle_filter_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.filter_mode = false;
                self.rebuild_rows();
                self.anchor_first_selectable();
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Enter => {
                self.filter_mode = false;
                ModelSelectorEvent::Consumed
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.rebuild_rows();
                self.anchor_first_selectable();
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.rebuild_rows();
                self.anchor_first_selectable();
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            _ => ModelSelectorEvent::Consumed,
        }
    }
}
