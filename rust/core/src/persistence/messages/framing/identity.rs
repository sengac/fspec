use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::Deserializer;
use std::fmt;
use uuid::Uuid;

struct IdentityVisitor<'identity>(&'identity mut Option<Uuid>);

impl<'de> Visitor<'de> for IdentityVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a message object")
    }

    fn visit_map<Access>(self, mut access: Access) -> Result<Self::Value, Access::Error>
    where
        Access: MapAccess<'de>,
    {
        while let Some(key) = access.next_key::<String>()? {
            if key == "id" {
                *self.0 = Some(access.next_value::<Uuid>()?);
                return Ok(());
            }
            access.next_value::<IgnoredAny>()?;
        }
        Ok(())
    }
}

pub(super) fn before_framing_failure(bytes: &[u8]) -> Option<Uuid> {
    let mut identity = None;
    let _ =
        serde_json::Deserializer::from_slice(bytes).deserialize_map(IdentityVisitor(&mut identity));
    identity
}
