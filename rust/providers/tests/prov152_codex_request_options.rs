//! Feature: spec/features/codex-capability-aware-request-options.feature

use codelet_providers::codex::request_options::{
    resolve, CodexCapabilities, CodexRequestOptions, RequestOptionsError,
};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn capabilities() -> Result<CodexCapabilities, serde_json::Error> {
    serde_json::from_value(json!({
        "efforts": ["none", "minimal", "low", "medium", "high", "xhigh", 2048],
        "contexts": ["auto", "current_turn", "all_turns"],
        "summaries": ["auto", "concise", "detailed"],
        "verbosities": ["low", "medium", "high"],
        "service_tiers": ["auto", "priority"],
        "sequential_cutoff": true
    }))
}

#[test]
fn scenario_advertised_effort_levels_retain_their_wire_representation() -> TestResult {
    // @step Given fresh model capabilities advertise named and custom numeric effort levels
    // @step When each supported effort is explicitly requested
    // @step Then named efforts including xhigh serialize as strings
    // @step And custom numeric efforts serialize as numbers without downgrade
    let caps = capabilities()?;
    for effort in [
        json!("none"),
        json!("minimal"),
        json!("low"),
        json!("medium"),
        json!("high"),
        json!("xhigh"),
        json!(2048),
        json!("2048"),
    ] {
        let result = resolve(Some(&json!({"reasoning": {"effort": effort}})), Some(&caps))?;
        let expected = if effort == "2048" {
            json!(2048)
        } else {
            effort
        };
        assert_eq!(result["reasoning"]["effort"], expected);
    }
    Ok(())
}

#[test]
fn scenario_invalid_explicit_options_fail_with_actionable_typed_errors() -> TestResult {
    // @step Given a model advertises a restricted set of options
    // @step When unsupported efforts malformed fields or unknown options are requested
    // @step Then each request fails with an option-specific typed error before transport
    let caps = capabilities()?;
    for config in [
        json!({"reasoning":{"effort":"ultra"}}),
        json!({"reasoning":{"effort":4096}}),
        json!({"service_tier":"flex"}),
    ] {
        let error = resolve(Some(&config), Some(&caps))
            .err()
            .ok_or("unsupported option must fail")?;
        assert!(matches!(error, RequestOptionsError::Unsupported { .. }));
        assert!(error.to_string().contains("supported"));
    }
    for config in [
        json!(true),
        json!([]),
        json!({"reasoning":true}),
        json!({"reasoning":{"effort":-1}}),
        json!({"reasoning":{"effort":0.5}}),
        json!({"reasoning":{"effrot":"high"}}),
        json!({"text":{"verbosity":2}}),
        json!({"stream_options":{"reasoning_summary_delivery":"parallel"}}),
        json!({"store":"false"}),
        json!({"top_p":"bad"}),
        json!({"typo":true}),
    ] {
        let error = resolve(Some(&config), Some(&caps))
            .err()
            .ok_or("malformed option must fail")?;
        assert!(matches!(error, RequestOptionsError::Invalid { .. }));
        assert!(error.to_string().contains("Codex"));
    }
    Ok(())
}

#[test]
fn scenario_optional_controls_require_advertised_capabilities() -> TestResult {
    // @step Given a model advertises context summary verbosity service tier and sequential-cutoff delivery
    // @step When the user explicitly enables those controls
    // @step Then each supported control appears in the request
    // @step And the same controls are rejected by a model without those capabilities
    let caps = capabilities()?;
    let config = json!({
        "reasoning": {"context":"all_turns", "summary":"detailed"},
        "text":{"verbosity":"high"}, "service_tier":"priority",
        "stream_options":{"reasoning_summary_delivery":"sequential_cutoff"}
    });
    let result = resolve(Some(&config), Some(&caps))?;
    for (key, value) in config.as_object().ok_or("object expected")? {
        assert_eq!(&result[key], value);
    }
    let unsupported = CodexCapabilities::default();
    for config in [
        json!({"reasoning":{"context":"auto"}}),
        json!({"reasoning":{"summary":"auto"}}),
        json!({"text":{"verbosity":"low"}}),
        json!({"service_tier":"auto"}),
        json!({"stream_options":{"reasoning_summary_delivery":"sequential_cutoff"}}),
    ] {
        assert!(matches!(
            resolve(Some(&config), Some(&unsupported)),
            Err(RequestOptionsError::Unsupported { .. })
        ));
    }
    Ok(())
}

#[test]
fn scenario_verbosity_does_not_require_a_text_format() -> TestResult {
    // @step Given a model advertises text verbosity
    // @step When the user configures only verbosity
    // @step Then the request contains text verbosity without a fabricated format
    let result = resolve(
        Some(&json!({"text":{"verbosity":"low"}})),
        Some(&capabilities()?),
    )?;
    assert_eq!(result["text"], json!({"verbosity":"low"}));
    Ok(())
}

#[test]
fn scenario_missing_and_stale_capabilities_use_a_conservative_fallback() -> TestResult {
    // @step Given capability data is missing or stale
    // @step When requests use defaults or explicitly request experimental options
    // @step Then defaults contain only baseline effort and required Codex fields
    // @step And explicit auto summaries retain established Codex compatibility
    // @step And experimental options are rejected until fresh capabilities advertise them
    let mut stale = capabilities()?;
    stale.stale = true;
    stale.defaults =
        serde_json::from_value(json!({"reasoning":{"effort":"xhigh","summary":"auto"}}))?;
    for caps in [None, Some(&stale)] {
        assert_eq!(
            resolve(None, caps)?,
            json!({"store":false,
            "include":["reasoning.encrypted_content"], "reasoning":{"effort":"high"}})
        );
        assert_eq!(
            resolve(Some(&json!({"reasoning":{"summary":"auto"}})), caps)?["reasoning"],
            json!({"effort":"high","summary":"auto"})
        );
        for config in [
            json!({"reasoning":{"effort":"xhigh"}}),
            json!({"reasoning":{"summary":"concise"}}),
            json!({"reasoning":{"summary":"detailed"}}),
            json!({"text":{"verbosity":"high"}}),
        ] {
            assert!(resolve(Some(&config), caps).is_err());
        }
    }
    Ok(())
}

#[test]
fn scenario_explicit_configuration_overrides_supported_model_defaults() -> TestResult {
    // @step Given model defaults include supported and unsupported optional controls
    // @step When the user overrides one supported field
    // @step Then the snapshot preserves other supported defaults and the explicit override
    // @step And unsupported defaults are omitted while explicit unsupported values fail
    // @step And required Codex fields cannot be overridden
    let mut caps = capabilities()?;
    caps.summaries.clear();
    caps.defaults = serde_json::from_value(json!({
        "reasoning":{"effort":"low","summary":"auto","context":"current_turn"},
        "text":{"verbosity":"medium"},"service_tier":"priority"
    }))?;
    let result = resolve(
        Some(&json!({"reasoning":{"effort":"xhigh"},
        "store":true,"include":["message.output_text.logprobs"]})),
        Some(&caps),
    )?;
    insta::assert_yaml_snapshot!(result, @r#"
    reasoning:
      effort: xhigh
      context: current_turn
    text:
      verbosity: medium
    service_tier: priority
    include:
      - message.output_text.logprobs
      - reasoning.encrypted_content
    store: false
    "#);
    assert!(resolve(Some(&json!({"reasoning":{"summary":"auto"}})), Some(&caps)).is_err());
    Ok(())
}

#[test]
fn scenario_capability_refresh_affects_the_next_request() -> TestResult {
    // @step Given a capability snapshot does not support a requested effort
    // @step When fresh capabilities advertise that effort
    // @step Then the next request accepts the effort without changing explicit configuration
    let config = json!({"reasoning":{"effort":"xhigh"}});
    assert!(resolve(Some(&config), Some(&CodexCapabilities::default())).is_err());
    assert_eq!(
        resolve(Some(&config), Some(&capabilities()?))?["reasoning"]["effort"],
        "xhigh"
    );
    Ok(())
}

fn arbitrary_json() -> impl proptest::strategy::Strategy<Value = Value> {
    use proptest::prelude::*;
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|number| json!(number)),
        ".*".prop_map(Value::String)
    ]
    .prop_recursive(4, 64, 8, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
            proptest::collection::btree_map(".*", inner, 0..8)
                .prop_map(|fields| Value::Object(fields.into_iter().collect()))
        ]
    })
}

proptest::proptest! {
    #[test]
    fn scenario_arbitrary_json_option_parsing_never_panics(value in arbitrary_json(), text in ".*") {
        // @step Given arbitrary nested JSON values and JSON text
        // @step When Codex parses and resolves the options
        // @step Then parsing returns resolved options or a typed error without panicking
        let _ = CodexRequestOptions::parse(&value);
        let _ = serde_json::from_str::<CodexRequestOptions>(&text);
        let _ = resolve(Some(&value), None);
        for field in ["reasoning", "text", "service_tier", "stream_options", "include", "store"] {
            let _ = resolve(Some(&json!({field: value.clone()})), None);
        }
    }
}
