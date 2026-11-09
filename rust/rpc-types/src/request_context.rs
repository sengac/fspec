use crate::ContextFillInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextProvenance {
    Measured,
    Estimated,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContextSnapshot {
    pub model: String,
    pub generation: i64,
    pub request: u64,
    #[serde(default)]
    pub physical_request: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub capacity: Option<u64>,
    pub threshold: Option<u64>,
    pub provenance: ContextProvenance,
}

impl RequestContextSnapshot {
    /// The context fill, `Ok(None)` when the limits or usage are unknown.
    /// BUG-462: an occupancy or fill percentage that does not fit its field
    /// is an error naming it, never a saturated number.
    pub fn context_fill(&self) -> Result<Option<ContextFillInfo>, String> {
        let Some((input, output, threshold, capacity)) = self.known_fill() else {
            return Ok(None);
        };
        let occupancy = input.checked_add(output).ok_or_else(|| {
            format!("context occupancy {input} + {output} tokens does not fit 64 bits")
        })?;
        let percentage = u128::from(occupancy) * 100 / u128::from(threshold);
        let fill_percentage = u32::try_from(percentage).map_err(|_| {
            format!("context fill {percentage}% does not fit the 32-bit field that reports it")
        })?;
        Ok(Some(ContextFillInfo {
            fill_percentage,
            effective_tokens: occupancy as f64,
            threshold: threshold as f64,
            context_window: capacity as f64,
        }))
    }

    /// Input, output, threshold and capacity when all are known and usable.
    fn known_fill(&self) -> Option<(u64, u64, u64, u64)> {
        if self.provenance == ContextProvenance::Unknown || self.model.is_empty() {
            return None;
        }
        let capacity = self.capacity.filter(|capacity| *capacity > 0)?;
        let threshold = self
            .threshold
            .filter(|threshold| *threshold > 0 && *threshold <= capacity)?;
        Some((self.input_tokens?, self.output_tokens?, threshold, capacity))
    }
}
