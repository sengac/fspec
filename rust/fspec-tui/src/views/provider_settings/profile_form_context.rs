use codelet_rpc_types::ProfileDefinition;

use super::profile_form::ProfileForm;
use super::profile_form_parse::opt_num;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileFormContext {
    pub settings: bool,
    pub inherited_auth: bool,
    pub streaming: Option<bool>,
    pub preserve_thinking: Option<bool>,
    pub loop_detection: Option<bool>,
}

impl ProfileForm {
    pub fn new_for_provider(provider_id: &str) -> Self {
        let mut form = Self::new_create();
        if provider_id != "openai" {
            form.base_url.clear();
            form.context.inherited_auth = true;
            form.preserve_thinking = matches!(provider_id, "anthropic" | "gemini");
        }
        form
    }

    pub fn from_provider_definition(
        provider_id: &str,
        name: &str,
        definition: &ProfileDefinition,
    ) -> Self {
        let mut form = Self::from_definition(name, definition);
        if provider_id != "openai" {
            form.context.inherited_auth = true;
            form.apply_optional_defaults(provider_id, definition);
        }
        form
    }

    pub fn from_settings(provider_id: &str, definition: &ProfileDefinition) -> Self {
        let mut form = Self::from_definition("", definition);
        form.context.settings = true;
        form.context.inherited_auth = true;
        form.field_index = 2;
        form.apply_optional_defaults(provider_id, definition);
        form
    }

    fn apply_optional_defaults(&mut self, provider_id: &str, definition: &ProfileDefinition) {
        self.max_images = opt_num(definition.max_images);
        self.loop_window = opt_num(definition.loop_detection_window);
        self.loop_repeat = opt_num(definition.loop_detection_max_repeats);
        self.loop_retries = opt_num(definition.loop_detection_max_retries);
        self.preserve_thinking = definition
            .preserve_thinking
            .unwrap_or(matches!(provider_id, "anthropic" | "gemini"));
    }

    pub(super) fn first_field(&self) -> usize {
        if self.context.settings {
            2
        } else {
            0
        }
    }
}
