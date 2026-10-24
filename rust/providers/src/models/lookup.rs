use super::ModelInfo;
use std::collections::HashMap;

pub(super) fn find_model<'models>(
    models: &'models HashMap<String, ModelInfo>,
    model: &str,
) -> Option<&'models ModelInfo> {
    if let Some(found) = models.get(model) {
        return Some(found);
    }
    let mut exact = models.values().filter(|definition| definition.id == model);
    if let Some(found) = exact.next() {
        return exact.next().is_none().then_some(found);
    }
    let mut prefixes = models
        .iter()
        .filter(|(key, definition)| key.starts_with(model) || definition.id.starts_with(model));
    let (_, found) = prefixes.next()?;
    prefixes.next().is_none().then_some(found)
}
