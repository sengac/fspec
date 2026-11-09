use super::ModelInfo;
use std::collections::HashMap;

/// Find a catalog model by its catalog key or, failing that, by a unique
/// exact `id`. BUG-335: a name that only prefixes a catalog model (`gpt-4`
/// against `gpt-4o`) matches nothing, so it never borrows that model's
/// capacity.
pub(super) fn find_model<'models>(
    models: &'models HashMap<String, ModelInfo>,
    model: &str,
) -> Option<&'models ModelInfo> {
    if let Some(found) = models.get(model) {
        return Some(found);
    }
    let mut exact = models.values().filter(|definition| definition.id == model);
    let found = exact.next()?;
    exact.next().is_none().then_some(found)
}
