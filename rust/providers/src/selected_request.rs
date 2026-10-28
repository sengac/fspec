use crate::request_admission::{
    admit_complete_request, AdmissionLimits, AdmissionProvenance, RequestAdmissionError,
};
use rig::completion::{CompletionModel, CompletionRequest};
use rig::message::Message;
use serde_json::Value;
use std::sync::Arc;

type Prepare =
    dyn Fn(&[Message], Option<u64>) -> Result<Vec<u8>, RequestAdmissionError> + Send + Sync;

#[derive(Clone)]
pub struct SelectedRequestPreparer {
    serialize: Arc<Prepare>,
    limits: AdmissionLimits,
}

impl std::fmt::Debug for SelectedRequestPreparer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SelectedRequestPreparer")
    }
}

#[derive(Debug)]
pub struct PreparedSelectedRequest {
    bytes: Vec<u8>,
    input_tokens: u64,
    output_reservation: u64,
    generation: u64,
}

impl PreparedSelectedRequest {
    pub fn request_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn input_tokens(&self) -> u64 {
        self.input_tokens
    }

    pub fn output_reservation(&self) -> u64 {
        self.output_reservation
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn provenance(&self) -> AdmissionProvenance {
        AdmissionProvenance::Estimated
    }
}

impl SelectedRequestPreparer {
    pub fn new<M: CompletionModel + 'static>(
        model: M,
        template: CompletionRequest,
        limits: AdmissionLimits,
    ) -> Self {
        Self {
            limits,
            serialize: Arc::new(move |messages, generation| {
                if generation == Some(0) {
                    return Err(RequestAdmissionError::Preparation(
                        "a replacement requires an advancing generation".into(),
                    ));
                }
                let mut request = template.clone();
                request.chat_history = rig::OneOrMany::many(messages.to_vec())
                    .map_err(|error| RequestAdmissionError::Preparation(error.to_string()))?;
                match generation {
                    Some(generation) => model.prepare_request_for_generation(request, generation),
                    None => model.prepare_request(request),
                }
                .map_err(|error| RequestAdmissionError::Preparation(error.to_string()))
            }),
        }
    }

    pub fn map_history(
        self,
        transform: impl Fn(&[Message]) -> Vec<Message> + Send + Sync + 'static,
    ) -> Self {
        Self {
            limits: self.limits,
            serialize: Arc::new(move |messages, generation| {
                (self.serialize)(&transform(messages), generation)
            }),
        }
    }

    pub fn prepare(
        &self,
        messages: &[Message],
        generation: u64,
    ) -> Result<PreparedSelectedRequest, RequestAdmissionError> {
        let bytes = (self.serialize)(messages, Some(generation))?;
        let admitted = admit_complete_request(&bytes, self.limits)?;
        Ok(PreparedSelectedRequest {
            input_tokens: admitted.input_tokens(),
            output_reservation: admitted.output_reservation(),
            bytes,
            generation,
        })
    }

    pub fn validate_current_history(
        &self,
        messages: &[Message],
    ) -> Result<(), RequestAdmissionError> {
        (self.serialize)(messages, None).map(|_| ())
    }
}

pub fn media_occupancy(value: &Value) -> Option<u64> {
    crate::request_admission::media::estimate(&mut value.clone(), true)
        .ok()
        .map(|estimate| estimate.tokens)
}
