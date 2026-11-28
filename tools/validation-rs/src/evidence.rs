use crate::Result;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{fmt, fs, io, io::Write, path::Path};
use tempfile::NamedTempFile;

pub fn digest(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    io::copy(&mut fs::File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

struct Strict(Value);
struct JsonVisitor;

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        d.deserialize_any(JsonVisitor).map(Strict)
    }
}

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("finite JSON with unique keys")
    }
    fn visit_unit<E>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E>(self, v: bool) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_str<E>(self, v: &str) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E>(self, v: u64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E>(self, v: i64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite number"))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(Strict(x)) = seq.next_element()? {
            items.push(x);
        }
        Ok(Value::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> std::result::Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some((key, Strict(value))) = entries.next_entry::<String, Strict>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key {key}")));
            }
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}

pub fn strict(bytes: &[u8]) -> Result<Value> {
    Ok(serde_json::from_slice::<Strict>(bytes)?.0)
}

pub fn atomic(path: &Path, value: &Value) -> Result<()> {
    let mut tmp = NamedTempFile::new_in(path.parent().ok_or("missing report directory")?)?;
    serde_json::to_writer_pretty(&mut tmp, value)?;
    writeln!(tmp)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_invalid_evidence() {
        for raw in [
            r#"[{"h":1,"\u0068":2}]"#,
            "[1e999]",
            "[NaN]",
            "[Infinity]",
            "null false",
        ] {
            assert!(strict(raw.as_bytes()).is_err(), "{raw}");
        }
        let v = strict(br#"[null, -0.0, 5e-324]"#).unwrap();
        assert!(v[0].is_null());
        assert_eq!(v[1].as_f64().unwrap().to_bits(), (-0.0f64).to_bits());
        assert_eq!(v[2].as_f64().unwrap().to_bits(), 1);
    }
}
