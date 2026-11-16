//! Shared test helpers.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Debug;

/// Parses literal JSON into `T`, checks it equals `expected`, then serialises it
/// back and checks the JSON equals the literal (as values, so key order and
/// whitespace do not matter). A renamed, missing or extra field fails.
pub fn roundtrip<T>(literal: &str, expected: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let parsed: T = serde_json::from_str(literal)
        .unwrap_or_else(|e| panic!("literal does not parse: {e}\n{literal}"));
    assert_eq!(&parsed, expected, "parsed value differs");
    let back = serde_json::to_value(&parsed).expect("serialise");
    let lit: Value = serde_json::from_str(literal).expect("literal is JSON");
    assert_eq!(back, lit, "serialised JSON differs from the literal");
    assert_eq!(
        serde_json::to_value(expected).expect("serialise"),
        lit,
        "expected value does not serialise to the literal"
    );
}
