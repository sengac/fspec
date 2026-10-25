use rig::completion::GetTokenUsage;
use rig::providers::gemini::completion::gemini_api_types::{FinishReason, PartKind};
use rig::providers::gemini::streaming::{
    PartialUsage, StreamGenerateContentResponse, StreamingCompletionResponse,
};

#[path = "../../../patches/rig-core/src/providers/gemini/streaming/deserialization_tests.rs"]
mod deserialization_tests;

#[path = "../../../patches/rig-core/src/providers/gemini/streaming/usage_tests.rs"]
mod usage_tests;
