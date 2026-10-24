use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize, Serialize)]
#[serde(tag = "responseAttempt", rename_all = "camelCase", deny_unknown_fields)]
enum Started {
    Started,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "responseAttempt", rename_all = "camelCase", deny_unknown_fields)]
enum Aborted {
    Aborted,
}

pub(super) fn serialize_started<Output: Serializer>(
    output: Output,
) -> Result<Output::Ok, Output::Error> {
    Started::Started.serialize(output)
}

pub(super) fn serialize_aborted<Output: Serializer>(
    output: Output,
) -> Result<Output::Ok, Output::Error> {
    Aborted::Aborted.serialize(output)
}

pub(super) fn deserialize_started<'de, Input: Deserializer<'de>>(
    input: Input,
) -> Result<(), Input::Error> {
    Started::deserialize(input).map(|_| ())
}

pub(super) fn deserialize_aborted<'de, Input: Deserializer<'de>>(
    input: Input,
) -> Result<(), Input::Error> {
    Aborted::deserialize(input).map(|_| ())
}
