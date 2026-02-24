use crate::save::rawdata::{passthrough_decode, passthrough_encode};
use serde_json::Value;

pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    Ok(passthrough_decode(bytes))
}

pub fn encode(value: &Value) -> Result<Vec<u8>, String> {
    passthrough_encode(value)
}
