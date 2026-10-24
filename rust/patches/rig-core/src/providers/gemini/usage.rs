use crate::completion::Usage;

pub(super) fn normalize(
    input: i32,
    candidates: Option<i32>,
    thoughts: Option<i32>,
    cached: Option<i32>,
    total: i32,
) -> Option<Usage> {
    let input = u64::try_from(input).ok()?;
    let candidates = u64::try_from(candidates?).ok()?;
    let reasoning = thoughts.map(u64::try_from).transpose().ok()?;
    let cached = cached.map(u64::try_from).transpose().ok()?;
    Usage::from_inclusive(
        input,
        candidates.checked_add(reasoning.unwrap_or(0))?,
        u64::try_from(total).ok()?,
        cached,
        reasoning,
    )
}
