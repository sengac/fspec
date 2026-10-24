use thiserror::Error;

#[path = "request_media.rs"]
pub(crate) mod media;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionLimits {
    pub context_capacity: Option<u64>,
    pub output_reservation: Option<u64>,
}

impl AdmissionLimits {
    pub fn validate(self) -> Result<(), RequestAdmissionError> {
        let capacity = self
            .context_capacity
            .filter(|capacity| *capacity > 0)
            .ok_or(RequestAdmissionError::MissingContextCapacity)?;
        let reservation = self
            .output_reservation
            .filter(|reservation| *reservation > 0)
            .ok_or(RequestAdmissionError::MissingOutputReservation)?;
        if reservation >= capacity {
            return Err(RequestAdmissionError::InvalidLimits {
                capacity,
                reservation,
            });
        }
        Ok(())
    }
}

pub(crate) fn validate_output_reservation(
    request: &serde_json::Value,
    limits: AdmissionLimits,
) -> Result<(), RequestAdmissionError> {
    let reservation = limits
        .output_reservation
        .filter(|reservation| *reservation > 0)
        .ok_or(RequestAdmissionError::MissingOutputReservation)?;
    for allowance in [
        request.get("max_output_tokens"),
        request.get("max_completion_tokens"),
        request.get("max_tokens"),
        request.pointer("/generationConfig/maxOutputTokens"),
        request.pointer("/generation_config/max_output_tokens"),
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.is_null())
    {
        let allowance = allowance.as_u64().ok_or_else(|| {
            RequestAdmissionError::Preparation(
                "wire output allowance must be an unsigned token count".into(),
            )
        })?;
        if allowance > reservation {
            return Err(RequestAdmissionError::OutputReservationMismatch {
                allowance,
                reservation,
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionProvenance {
    Estimated,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RequestAdmissionError {
    #[error("selected request preparation unavailable: {0}")]
    Preparation(String),
    #[error("request admission requires an explicit positive context capacity")]
    MissingContextCapacity,
    #[error("request admission requires an explicit positive output reservation")]
    MissingOutputReservation,
    #[error("wire output allowance {allowance} exceeds output reservation {reservation}")]
    OutputReservationMismatch { allowance: u64, reservation: u64 },
    #[error("output reservation {reservation} leaves no input budget in capacity {capacity}")]
    InvalidLimits { capacity: u64, reservation: u64 },
    #[error("request admission requires known media occupancy")]
    UnknownMediaOccupancy,
    #[error("request occupancy exceeds the representable token count")]
    OccupancyOverflow,
    #[error("complete request input {input_tokens} plus output reservation {output_reservation} exceeds capacity {context_capacity}")]
    BudgetExceeded {
        input_tokens: u64,
        output_reservation: u64,
        context_capacity: u64,
    },
}

#[derive(Debug)]
pub struct AdmittedRequest<'request> {
    request: &'request [u8],
    input_tokens: u64,
    output_reservation: u64,
}

impl AdmittedRequest<'_> {
    pub fn request_bytes(&self) -> &[u8] {
        self.request
    }

    pub fn provenance(&self) -> AdmissionProvenance {
        AdmissionProvenance::Estimated
    }

    pub fn input_tokens(&self) -> u64 {
        self.input_tokens
    }

    pub fn output_reservation(&self) -> u64 {
        self.output_reservation
    }
}

pub fn admit_serialized_request(
    request: &[u8],
    limits: AdmissionLimits,
    media_tokens: Option<u64>,
) -> Result<AdmittedRequest<'_>, RequestAdmissionError> {
    limits.validate()?;
    let text = std::str::from_utf8(request)
        .map_err(|error| RequestAdmissionError::Preparation(error.to_string()))?;
    admit_with_media(request, limits, media_tokens, text)
}

pub fn admit_complete_request(
    request: &[u8],
    limits: AdmissionLimits,
) -> Result<AdmittedRequest<'_>, RequestAdmissionError> {
    limits.validate()?;
    let mut value = serde_json::from_slice(request)
        .map_err(|error| RequestAdmissionError::Preparation(error.to_string()))?;
    validate_output_reservation(&value, limits)?;
    let estimate = media::estimate(&mut value, true)?;
    admit_with_media(request, limits, Some(estimate.tokens), &value.to_string())
}

fn admit_with_media<'request>(
    request: &'request [u8],
    limits: AdmissionLimits,
    media_tokens: Option<u64>,
    text: &str,
) -> Result<AdmittedRequest<'request>, RequestAdmissionError> {
    limits.validate()?;
    let capacity = limits
        .context_capacity
        .filter(|capacity| *capacity > 0)
        .ok_or(RequestAdmissionError::MissingContextCapacity)?;
    let reservation = limits
        .output_reservation
        .filter(|reservation| *reservation > 0)
        .ok_or(RequestAdmissionError::MissingOutputReservation)?;
    let media_tokens = media_tokens.ok_or(RequestAdmissionError::UnknownMediaOccupancy)?;
    let text_tokens = u64::try_from(codelet_common::token_estimator::count_tokens(text))
        .map_err(|_| RequestAdmissionError::OccupancyOverflow)?;
    let input_tokens = text_tokens
        .checked_add(media_tokens)
        .ok_or(RequestAdmissionError::OccupancyOverflow)?;
    if input_tokens > capacity - reservation {
        return Err(RequestAdmissionError::BudgetExceeded {
            input_tokens,
            output_reservation: reservation,
            context_capacity: capacity,
        });
    }
    Ok(AdmittedRequest {
        request,
        input_tokens,
        output_reservation: reservation,
    })
}
