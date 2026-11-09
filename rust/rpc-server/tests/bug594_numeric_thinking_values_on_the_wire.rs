//! BUG-594: numeric thinking values cross the bincode RPC wire as tagged
//! numbers (u32 tag 0 = Whole + i64, 1 = Decimal + f64); JSON keeps the
//! plain number.
//!
//! Feature: spec/features/numeric-thinking-values-cross-the-rpc-wire.feature

use codelet_rpc_types::{NumericEffortKind, ThinkingChoice, ThinkingLevel, ThinkingOptions};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Number};
use std::fmt::Debug;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn decimal(value: f64) -> Result<Number, String> {
    Number::from_f64(value).ok_or_else(|| format!("{value} is not a JSON number"))
}

/// The bincode bytes of `value`, decoded back and checked equal.
fn round_trip<T>(value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>>
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = bincode::serialize(value)?;
    let decoded: T = bincode::deserialize(&bytes)?;
    assert_eq!(&decoded, value, "bytes {bytes:?}");
    Ok(bytes)
}

fn wire(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

#[test]
fn a_decimal_thinking_level_crosses_the_wire_as_a_tagged_decimal() -> TestResult {
    // @step Given the thinking level Numeric 0.5
    let level = ThinkingLevel::Numeric(decimal(0.5)?);

    // @step When it is encoded with bincode
    let bytes = bincode::serialize(&level)?;

    // @step Then the bytes are 4,0,0,0, 1,0,0,0, 0,0,0,0,0,0,224,63
    assert_eq!(bytes, [4, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 224, 63]);

    // @step And decoding the bytes gives Numeric 0.5
    assert_eq!(bincode::deserialize::<ThinkingLevel>(&bytes)?, level);
    Ok(())
}

#[test]
fn a_whole_thinking_level_crosses_the_wire_as_a_tagged_whole() -> TestResult {
    // @step Given the thinking levels Numeric 3 and Numeric -2
    let three = ThinkingLevel::Numeric(Number::from(3));
    let minus_two = ThinkingLevel::Numeric(Number::from(-2));

    // @step When each is encoded with bincode
    let three_bytes = bincode::serialize(&three)?;
    let minus_two_bytes = bincode::serialize(&minus_two)?;

    // @step Then Numeric 3 is the bytes 4,0,0,0, 0,0,0,0, 3,0,0,0,0,0,0,0
    assert_eq!(
        three_bytes,
        [4, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        minus_two_bytes,
        wire(&[&[4, 0, 0, 0, 0, 0, 0, 0], &(-2i64).to_le_bytes()])
    );

    // @step And each decodes back to the same whole number
    let decoded_three = bincode::deserialize::<ThinkingLevel>(&three_bytes)?;
    let decoded_minus_two = bincode::deserialize::<ThinkingLevel>(&minus_two_bytes)?;
    assert_eq!(decoded_three, three);
    assert_eq!(decoded_minus_two, minus_two);
    for decoded in [decoded_three, decoded_minus_two] {
        let ThinkingLevel::Numeric(number) = decoded else {
            return Err(format!("not numeric: {decoded:?}").into());
        };
        assert!(number.is_i64(), "{number} is not whole");
    }
    Ok(())
}

#[test]
fn a_numeric_effort_range_crosses_the_wire_with_tagged_bounds() -> TestResult {
    // @step Given the choice NumericEffort with kind Decimal, min 0 and max 1
    let choice = ThinkingChoice::NumericEffort {
        kind: NumericEffortKind::Decimal,
        min: Number::from(0),
        max: Number::from(1),
    };

    // @step When it is encoded with bincode
    let bytes = bincode::serialize(&choice)?;

    // @step Then the bytes are variant 2, kind 1, min Whole 0 and max Whole 1
    let expected = wire(&[
        &2u32.to_le_bytes(),
        &1u32.to_le_bytes(),
        &0u32.to_le_bytes(),
        &0i64.to_le_bytes(),
        &0u32.to_le_bytes(),
        &1i64.to_le_bytes(),
    ]);
    assert_eq!(bytes, expected);

    // @step And decoding the bytes gives the same choice
    assert_eq!(bincode::deserialize::<ThinkingChoice>(&bytes)?, choice);
    Ok(())
}

#[test]
fn every_thinking_level_choice_and_options_variant_round_trips_through_bincode() -> TestResult {
    // @step Given every ThinkingLevel variant, every ThinkingChoice variant and both ThinkingOptions variants
    let levels = [
        (ThinkingLevel::Off, 0u32),
        (ThinkingLevel::ModelDefault, 1),
        (ThinkingLevel::On, 2),
        (ThinkingLevel::Effort("xhigh".to_string()), 3),
        (ThinkingLevel::Numeric(decimal(0.5)?), 4),
        (ThinkingLevel::Numeric(Number::from(3)), 4),
        (ThinkingLevel::Budget(8192), 5),
    ];
    let choices = [
        (ThinkingChoice::Off, 0u32),
        (ThinkingChoice::Effort("high".to_string()), 1),
        (
            ThinkingChoice::NumericEffort {
                kind: NumericEffortKind::Whole,
                min: Number::from(1),
                max: Number::from(10),
            },
            2,
        ),
        (
            ThinkingChoice::NumericEffort {
                kind: NumericEffortKind::Decimal,
                min: decimal(0.25)?,
                max: decimal(0.75)?,
            },
            2,
        ),
        (
            ThinkingChoice::TokenBudget {
                min: Some(1024),
                max: None,
            },
            3,
        ),
        (
            ThinkingChoice::TokenBudget {
                min: None,
                max: Some(32000),
            },
            3,
        ),
        (ThinkingChoice::On, 4),
    ];
    let options = [
        (
            ThinkingOptions::NoReasoningControl {
                model: "local/plain".to_string(),
                reason: "no thinking capability".to_string(),
            },
            0u32,
        ),
        (
            ThinkingOptions::Levels {
                model: "custom/numeric".to_string(),
                choices: choices.iter().map(|(choice, _)| choice.clone()).collect(),
                takes_off: true,
            },
            1,
        ),
    ];

    // @step When each is encoded and decoded with bincode
    let mut encoded = Vec::new();
    for (level, index) in &levels {
        encoded.push((round_trip(level)?, *index));
    }
    for (choice, index) in &choices {
        encoded.push((round_trip(choice)?, *index));
    }
    for (option, index) in &options {
        encoded.push((round_trip(option)?, *index));
    }

    // @step Then each decodes to the value that was encoded
    assert_eq!(encoded.len(), levels.len() + choices.len() + options.len());

    // @step And each variant's first four bytes are its unchanged variant index
    for (bytes, index) in &encoded {
        assert_eq!(bytes.get(..4), Some(&index.to_le_bytes()[..]), "{bytes:?}");
    }
    Ok(())
}

#[test]
fn a_whole_number_that_does_not_fit_the_wire_is_an_encode_error() -> TestResult {
    // @step Given the thinking level Numeric 18446744073709551615
    let level = ThinkingLevel::Numeric(Number::from(u64::MAX));

    // @step When it is encoded with bincode
    let result = bincode::serialize(&level);

    // @step Then encoding fails with an error naming 18446744073709551615 that says it does not fit a whole number on the wire
    let Err(error) = result else {
        return Err(format!("encoded as {result:?}").into());
    };
    let message = error.to_string();
    assert!(
        message.contains("18446744073709551615")
            && message.contains("does not fit a whole number on the wire"),
        "unexpected error: {message}"
    );
    Ok(())
}

#[test]
fn a_decimal_that_is_not_finite_is_refused_when_decoding() -> TestResult {
    // @step Given the bytes of a Numeric thinking level tagged Decimal holding the f64 NaN
    let bytes = wire(&[
        &4u32.to_le_bytes(),
        &1u32.to_le_bytes(),
        &f64::NAN.to_le_bytes(),
    ]);

    // @step When they are decoded with bincode
    let result = bincode::deserialize::<ThinkingLevel>(&bytes);

    // @step Then decoding fails with an error saying the decimal is not finite
    let Err(error) = result else {
        return Err(format!("decoded as {result:?}").into());
    };
    let message = error.to_string();
    assert!(
        message.contains("NaN") && message.contains("is not a finite decimal"),
        "unexpected error: {message}"
    );
    Ok(())
}

#[test]
fn the_json_forms_keep_the_plain_number() -> TestResult {
    // @step Given the thinking level Numeric 0.5 and the choice NumericEffort with kind Whole, min 1 and max 10
    let level = ThinkingLevel::Numeric(decimal(0.5)?);
    let choice = ThinkingChoice::NumericEffort {
        kind: NumericEffortKind::Whole,
        min: Number::from(1),
        max: Number::from(10),
    };

    // @step When each is written as JSON
    let level_json = serde_json::to_string(&level)?;
    let choice_json = serde_json::to_string(&choice)?;

    // @step Then the level is {"Numeric":0.5}
    assert_eq!(level_json, r#"{"Numeric":0.5}"#);

    // @step And the choice is {"NumericEffort":{"kind":"Whole","min":1,"max":10}}
    assert_eq!(
        choice_json,
        r#"{"NumericEffort":{"kind":"Whole","min":1,"max":10}}"#
    );

    // @step And reading that JSON gives the same values
    assert_eq!(serde_json::from_str::<ThinkingLevel>(&level_json)?, level);
    assert_eq!(
        serde_json::from_str::<ThinkingChoice>(&choice_json)?,
        choice
    );
    assert_eq!(
        serde_json::from_value::<ThinkingLevel>(json!({"Numeric": 0.5}))?,
        level
    );
    assert_eq!(
        serde_json::to_value(&choice)?,
        serde_json::from_str::<serde_json::Value>(&choice_json)?
    );
    Ok(())
}
