//! Small helpers that reproduce JavaScript's loose value rules.
//!
//! The saved state is plain JSON written by the web UI, so the engine reads it
//! the same way the UI's JavaScript does: `+x || 0`, truthiness, `Math.round`.
//! Matching these rules exactly is what keeps the Rust engine's numbers
//! identical to the web version's.

use serde_json::{Map, Value};

/// `obj[key]`, or `None` when `obj` isn't an object or the key is missing.
pub fn get<'a>(obj: &'a Value, key: &str) -> Option<&'a Value> {
    obj.as_object().and_then(|m| m.get(key))
}

/// JavaScript unary plus (`+v`), with a missing value as `undefined`.
pub fn num(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Some(Value::Array(a)) if a.is_empty() => 0.0,
        Some(_) => f64::NAN,
    }
}

/// `x || 0` for a number: NaN and 0 become 0.
pub fn or0(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x
    }
}

/// `+v || 0`.
pub fn num0(v: Option<&Value>) -> f64 {
    or0(num(v))
}

/// JavaScript truthiness.
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// `Math.round`: halves round up, toward +infinity.
pub fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// One decimal place, as the UI's `r1`.
pub fn r1(x: f64) -> f64 {
    round(x * 10.0) / 10.0
}

/// The string JavaScript would use as an object key for this value.
pub fn key(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "null".into(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
    }
}

/// A string field, or "" when missing or not a string.
pub fn s<'a>(obj: &'a Value, k: &str) -> &'a str {
    get(obj, k).and_then(Value::as_str).unwrap_or("")
}

/// An array field, or an empty slice.
pub fn arr<'a>(obj: &'a Value, k: &str) -> &'a [Value] {
    get(obj, k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// An object field, if it is one.
pub fn obj<'a>(o: &'a Value, k: &str) -> Option<&'a Map<String, Value>> {
    get(o, k).and_then(Value::as_object)
}

/// Turn an f64 back into JSON, keeping whole numbers as integers like JavaScript prints them.
pub fn jnum(x: f64) -> Value {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 9.0e15 {
        Value::from(x as i64)
    } else {
        serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unary_plus_matches_js() {
        assert_eq!(num(Some(&json!("45"))), 45.0);
        assert_eq!(num(Some(&json!(""))), 0.0);
        assert!(num(Some(&json!("abc"))).is_nan());
        assert!(num(None).is_nan());
        assert_eq!(num(Some(&json!(null))), 0.0);
        assert_eq!(num0(None), 0.0);
    }

    #[test]
    fn round_matches_math_round() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(r1(1.25), 1.3);
    }
}
