pub mod base_camp;
pub mod character;
pub mod character_container;
pub mod group;
pub mod work;
pub mod worker_director;

use byteorder::{LittleEndian, ReadBytesExt};
use gvas::cursor_ext::ReadExt;
use serde_json::{Value, json};
use std::io::Cursor;

pub fn normalize_guid(value: &str) -> String {
    if value == "0" {
        "00000000000000000000000000000000".to_string()
    } else {
        value.replace('-', "").to_uppercase()
    }
}

pub fn decode_guid(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    cursor
        .read_guid()
        .map(|guid| normalize_guid(&guid.to_string()))
        .map_err(|error| format!("failed to read guid: {error}"))
}

pub fn decode_fstring(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    cursor
        .read_fstring()
        .map_err(|error| format!("failed to read fstring: {error}"))?
        .ok_or_else(|| "failed to read fstring: value was null".to_string())
}

pub fn decode_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, String> {
    cursor
        .read_u8()
        .map_err(|error| format!("failed to read u8: {error}"))
}

pub fn decode_f32(cursor: &mut Cursor<&[u8]>) -> Result<f32, String> {
    cursor
        .read_f32::<LittleEndian>()
        .map_err(|error| format!("failed to read f32: {error}"))
}

pub fn read_bytes(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<Vec<u8>, String> {
    let start = cursor.position() as usize;
    let end = start.saturating_add(len);
    let slice = cursor
        .get_ref()
        .get(start..end)
        .ok_or_else(|| format!("failed to read {len} bytes"))?;
    cursor.set_position(end as u64);
    Ok(slice.to_vec())
}

pub fn read_remaining(cursor: &mut Cursor<&[u8]>) -> Vec<u8> {
    let position = cursor.position() as usize;
    cursor
        .get_ref()
        .get(position..)
        .map_or_else(Vec::new, |slice| slice.to_vec())
}

pub fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn from_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("hex length must be even".to_string());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let hi = decode_nibble(bytes[index])?;
        let lo = decode_nibble(bytes[index + 1])?;
        out.push((hi << 4) | lo);
        index += 2;
    }
    Ok(out)
}

fn decode_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex char: {}", byte as char)),
    }
}

pub fn passthrough_decode(bytes: &[u8]) -> Value {
    json!({
        "codec_status": "passthrough",
        "byte_len": bytes.len(),
        "original_bytes_hex": to_hex(bytes),
    })
}

pub fn passthrough_encode(value: &Value) -> Result<Vec<u8>, String> {
    let hex = value
        .get("original_bytes_hex")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing original_bytes_hex".to_string())?;
    from_hex(hex)
}
