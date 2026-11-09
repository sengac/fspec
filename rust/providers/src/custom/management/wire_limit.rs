//! PROV-172: declared token limits on the RPC wire.

use crate::custom::error::CustomProviderError;

/// PROV-172: convert a declared limit to the wire's `u32`, failing (never
/// clamping) when it does not fit.
pub(super) fn wire_limit(
    provider: &str,
    alias: &str,
    limit: &str,
    value: Option<usize>,
) -> Result<Option<u32>, CustomProviderError> {
    value
        .map(|value| {
            u32::try_from(value).map_err(|_| CustomProviderError::LimitExceedsWire {
                provider: provider.to_string(),
                field: format!("models.{alias}.{limit}"),
                value,
            })
        })
        .transpose()
}
