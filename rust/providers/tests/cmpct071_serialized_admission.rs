// Feature: spec/features/serialized-request-admission.feature
use codelet_providers::request_admission::{
    admit_serialized_request, AdmissionLimits, AdmissionProvenance, RequestAdmissionError,
};

fn limits(capacity: u64, reservation: u64) -> AdmissionLimits {
    AdmissionLimits {
        context_capacity: Some(capacity),
        output_reservation: Some(reservation),
    }
}

#[test]
fn unusable_limits_and_unrepresentable_occupancy_cannot_authorize_admission() {
    // @step Given complete selected request bytes and unavailable or unusable admission limits
    let selected = br#"{"instructions":"mandatory","input":"unchanged DAG","tools":[]}"#;
    let cases = [
        (
            AdmissionLimits {
                context_capacity: None,
                output_reservation: Some(256),
            },
            Some(0),
            RequestAdmissionError::MissingContextCapacity,
        ),
        (
            limits(0, 256),
            Some(0),
            RequestAdmissionError::MissingContextCapacity,
        ),
        (
            AdmissionLimits {
                context_capacity: Some(4_096),
                output_reservation: None,
            },
            Some(0),
            RequestAdmissionError::MissingOutputReservation,
        ),
        (
            limits(4_096, 0),
            Some(0),
            RequestAdmissionError::MissingOutputReservation,
        ),
        (
            limits(256, 256),
            Some(0),
            RequestAdmissionError::InvalidLimits {
                capacity: 256,
                reservation: 256,
            },
        ),
        (
            limits(256, 512),
            Some(0),
            RequestAdmissionError::InvalidLimits {
                capacity: 256,
                reservation: 512,
            },
        ),
        (
            limits(u64::MAX, 256),
            Some(u64::MAX),
            RequestAdmissionError::OccupancyOverflow,
        ),
    ];
    // @step When admission checks the limits and explicit media occupancy
    for (limits, media, expected) in cases {
        let result = admit_serialized_request(selected, limits, media);
        // @step Then every invalid budget returns its typed failure without changing request bytes
        assert_eq!(result.err(), Some(expected));
        assert_eq!(
            selected,
            br#"{"instructions":"mandatory","input":"unchanged DAG","tools":[]}"#
        );
    }
}

#[test]
fn serialized_schemas_and_retained_suffix_participate_in_the_same_budget(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a selected request containing instructions and a small unchanged DAG
    let mut request = serde_json::json!({
        "instructions": "mandatory instructions",
        "input": [{"role": "user", "content": "<dag-node>unchanged</dag-node>"}],
        "tools": []
    });
    let small = serde_json::to_vec(&request)?;
    let initial = admit_serialized_request(&small, limits(2_048, 256), Some(0));
    assert!(initial.is_ok(), "small complete request fits");
    // @step And large tool schemas and a retained conversation suffix
    request["tools"] = serde_json::json!([{"description": "large tool schema ".repeat(1000)}]);
    request["input"][0]["suffix"] = serde_json::json!("retained evidence ".repeat(1000));
    let selected = serde_json::to_vec(&request)?;
    let unchanged = selected.clone();
    // @step When the complete serialized request is admitted against explicit limits
    let result = admit_serialized_request(&selected, limits(2_048, 256), Some(0));
    // @step Then the schemas and suffix can veto admission without changing the request
    assert!(matches!(
        result,
        Err(RequestAdmissionError::BudgetExceeded { .. })
    ));
    assert_eq!(selected, unchanged);
    Ok(())
}

#[test]
fn unknown_media_occupancy_cannot_certify_request_fit() {
    // @step Given a selected request whose media occupancy is unavailable
    let selected = br#"{"input":[{"type":"input_image","image_url":"https://image.invalid"}]}"#;
    // @step When the complete serialized request is admitted against explicit limits
    let result = admit_serialized_request(selected, limits(200_000, 4_096), None);
    // @step Then admission returns a typed unavailable-media failure
    assert!(matches!(
        result,
        Err(RequestAdmissionError::UnknownMediaOccupancy)
    ));
}

#[test]
fn successful_estimated_admission_preserves_its_request_and_provenance(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a fitting selected request with explicit media occupancy and output reservation
    let selected = br#"{"instructions":"mandatory","input":"unchanged DAG","tools":[]}"#;
    // @step When the complete serialized request is admitted against explicit limits
    let admitted = admit_serialized_request(selected, limits(4_096, 256), Some(512))?;
    // @step Then admission retains the exact selected bytes and reports estimated occupancy
    assert_eq!(admitted.request_bytes(), selected);
    assert_eq!(admitted.provenance(), AdmissionProvenance::Estimated);
    assert_eq!(
        admitted.input_tokens(),
        codelet_common::token_estimator::count_tokens(std::str::from_utf8(selected)?) as u64 + 512
    );
    assert_eq!(admitted.output_reservation(), 256);

    use proptest::prelude::*;
    proptest::test_runner::TestRunner::default().run(
        &(".{1,1024}", 0u64..1_024),
        |(text, media)| {
            let capacity =
                codelet_common::token_estimator::count_tokens(&text) as u64 + media + 256;
            let admitted =
                admit_serialized_request(text.as_bytes(), limits(capacity, 256), Some(media))
                    .map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(admitted.request_bytes(), text.as_bytes());
            prop_assert_eq!(
                admitted.input_tokens() + admitted.output_reservation(),
                capacity
            );
            prop_assert_eq!(admitted.provenance(), AdmissionProvenance::Estimated);
            Ok(())
        },
    )?;
    Ok(())
}
