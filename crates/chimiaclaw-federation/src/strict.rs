//! Reject duplicate members before typed decoding. Round-trip equality also
//! rejects fields silently ignored by imported artifact types.
use crate::{Error, Result};
use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("strict JSON")
            }
            fn visit_bool<E>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Bool(v)))
            }
            fn visit_i64<E>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Number(v.into())))
            }
            fn visit_u64<E>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Number(v.into())))
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<Strict, E> {
                Err(E::custom("floating numbers forbidden"))
            }
            fn visit_str<E>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::String(v.into())))
            }
            fn visit_string<E>(self, v: String) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::String(v)))
            }
            fn visit_unit<E>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut out = Vec::new();
                while let Some(Strict(v)) = a.next_element()? {
                    out.push(v);
                }
                Ok(Strict(Value::Array(out)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if out.contains_key(&k) {
                        return Err(serde::de::Error::custom("duplicate member"));
                    }
                    let Strict(v) = a.next_value()?;
                    out.insert(k, v);
                }
                Ok(Strict(Value::Object(out)))
            }
        }
        d.deserialize_any(JsonVisitor)
    }
}

pub fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8], limit: usize) -> Result<T> {
    if bytes.len() > limit {
        return Err(Error("input_too_large"));
    }
    // Structural tokens outside strings bound allocation before Value parsing.
    // Large object bytes travel as strings and do not consume structural budget.
    let (mut quoted, mut escaped, mut structures) = (false, false, 0usize);
    for byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
        } else if *byte == b'"' {
            quoted = true;
        } else if b"[]{},:".contains(byte) {
            structures += 1;
            if structures > 20_000 {
                return Err(Error("json_structure_limit"));
            }
        }
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let Strict(value) =
        Strict::deserialize(&mut d).map_err(|_| Error("invalid_or_duplicate_json"))?;
    d.end().map_err(|_| Error("trailing_json"))?;
    let typed: T = serde_json::from_value(value.clone()).map_err(|_| Error("invalid_fields"))?;
    if serde_json::to_value(&typed).map_err(|_| Error("invalid_fields"))? != value {
        return Err(Error("unknown_or_noncanonical_fields"));
    }
    Ok(typed)
}
