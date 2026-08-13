use crate::model::{DiscoveryError, MAX_DOSSIER_BYTES, MAX_JSON_DEPTH, MAX_JSON_NODES};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt::Formatter;

struct ParseState {
    nodes: usize,
}

struct StrictSeed<'a> {
    state: &'a mut ParseState,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictSeed<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.depth > MAX_JSON_DEPTH {
            return Err(serde::de::Error::custom("json-depth-limit"));
        }
        self.state.nodes += 1;
        if self.state.nodes > MAX_JSON_NODES {
            return Err(serde::de::Error::custom("json-node-limit"));
        }
        deserializer.deserialize_any(StrictVisitor {
            state: self.state,
            depth: self.depth,
        })
    }
}

struct StrictVisitor<'a> {
    state: &'a mut ParseState,
    depth: usize,
}

impl<'de> Visitor<'de> for StrictVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("strict bounded JSON")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let value = i64::try_from(value).map_err(|_| E::custom("integer-out-of-range"))?;
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Err(E::custom("floating-number-not-allowed"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictSeed {
            state: self.state,
            depth: self.depth + 1,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom("duplicate-json-key"));
            }
            let value = map.next_value_seed(StrictSeed {
                state: self.state,
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

fn parse_error(error: &serde_json::Error) -> &'static str {
    let text = error.to_string();
    if text.contains("duplicate-json-key") {
        "duplicate-json-key"
    } else if text.contains("floating-number-not-allowed") {
        "floating-number-not-allowed"
    } else if text.contains("integer-out-of-range") {
        "integer-out-of-range"
    } else if text.contains("json-depth-limit") {
        "json-depth-limit"
    } else if text.contains("json-node-limit") {
        "json-node-limit"
    } else {
        "invalid-json"
    }
}

/// Parse one strict dossier-sized JSON value.
///
/// # Errors
///
/// Returns a stable error when the input exceeds a bound or violates the
/// restricted JSON grammar.
pub fn parse_strict_json(bytes: &[u8]) -> Result<Value, DiscoveryError> {
    parse_strict_json_with_limit(bytes, MAX_DOSSIER_BYTES)
}

pub(crate) fn parse_strict_json_with_limit(
    bytes: &[u8],
    maximum: usize,
) -> Result<Value, DiscoveryError> {
    if bytes.len() > maximum {
        return Err(DiscoveryError::new("input-too-large", "$"));
    }
    std::str::from_utf8(bytes).map_err(|_| DiscoveryError::new("invalid-utf8", "$"))?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut state = ParseState { nodes: 0 };
    let value = StrictSeed {
        state: &mut state,
        depth: 0,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| DiscoveryError::new(parse_error(&error), "$"))?;
    deserializer
        .end()
        .map_err(|_| DiscoveryError::new("trailing-json-data", "$"))?;
    Ok(value)
}
