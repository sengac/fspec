// Feature: spec/features/precise-lossless-coverage-mutations.feature
#[path = "refac020/support.rs"]
pub mod support;
#[path = "refac020_selected_coverage/readback.rs"]
mod readback;
#[path = "refac020_selected_coverage/linking.rs"]
mod linking;
#[path = "refac020_selected_coverage/unlinking.rs"]
mod unlinking;
#[path = "refac020_selected_coverage/rejection.rs"]
mod rejection;
#[path = "refac020_selected_coverage/validation.rs"]
mod validation;
#[path = "refac020_selected_coverage/ambiguity.rs"]
mod ambiguity;
