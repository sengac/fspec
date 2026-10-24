use crate::request_admission::{admit_complete_request, AdmissionLimits, RequestAdmissionError};
use rig::completion::CompletionError;

pub(super) fn admit(bytes: &[u8], limits: Option<AdmissionLimits>) -> Result<(), CompletionError> {
    let limits = limits.ok_or_else(|| {
        CompletionError::RequestError(Box::new(RequestAdmissionError::MissingContextCapacity))
    })?;
    admit_complete_request(bytes, limits)
        .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
    Ok(())
}
