//! CMPCT-088: narrowing counts for 32-bit wire and persisted fields.

/// `value` as a `u32`, or an error naming `what` and the count — never a
/// clamped or wrapped number.
pub fn narrow_count(what: &str, value: u64) -> Result<u32, String> {
    u32::try_from(value)
        .map_err(|_| format!("{what} count {value} does not fit the 32-bit field that reports it"))
}
