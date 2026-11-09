//! BUG-594: the wire form of a numeric thinking value.
//!
//! Feature: spec/features/numeric-thinking-values-cross-the-rpc-wire.feature
//!
//! `serde_json::Number` serializes as an untagged integer or float and
//! deserializes through `deserialize_any`, which bincode cannot decode, so
//! a [`ThinkingLevel::Numeric`](crate::ThinkingLevel::Numeric) or a
//! [`ThinkingChoice::NumericEffort`](crate::ThinkingChoice::NumericEffort)
//! bound could never cross the RPC wire. On a binary (not human-readable)
//! format the number is a tagged [`WireNumber`]: u32 tag 0 = `Whole` and an
//! i64, 1 = `Decimal` and an f64, the order of
//! [`NumericEffortKind`](crate::NumericEffortKind). Human-readable formats
//! (JSON) keep the plain number. Nothing is converted to make a value fit:
//! an integer above `i64::MAX` cannot be encoded and a decimal that is not
//! finite cannot be decoded.

use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Number;

/// A numeric thinking value as the binary wire carries it.
#[derive(Serialize, Deserialize)]
enum WireNumber {
    Whole(i64),
    Decimal(f64),
}

pub(crate) fn serialize<S: Serializer>(number: &Number, serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        return number.serialize(serializer);
    }
    let wire = match (number.as_i64(), number.is_f64()) {
        (Some(whole), _) => WireNumber::Whole(whole),
        (None, true) => WireNumber::Decimal(
            number
                .as_f64()
                .ok_or_else(|| S::Error::custom(format!("{number} has no f64 value")))?,
        ),
        (None, false) => {
            return Err(S::Error::custom(format!(
                "{number} does not fit a whole number on the wire (i64)"
            )))
        }
    };
    wire.serialize(serializer)
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Number, D::Error> {
    if deserializer.is_human_readable() {
        return Number::deserialize(deserializer);
    }
    match WireNumber::deserialize(deserializer)? {
        WireNumber::Whole(whole) => Ok(Number::from(whole)),
        WireNumber::Decimal(decimal) => Number::from_f64(decimal)
            .ok_or_else(|| D::Error::custom(format!("{decimal} is not a finite decimal"))),
    }
}
