use crate::save::rawdata::{decode_guid, decode_u8, passthrough_encode, read_remaining, to_hex};
use serde_json::{Value, json};
use std::io::Cursor;

pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    if bytes.is_empty() {
        return Ok(json!({
            "codec_status": "decoded",
            "is_empty": true,
            "original_bytes_hex": "",
        }));
    }

    let mut cursor = Cursor::new(bytes);
    let player_uid = decode_guid(&mut cursor)?;
    let instance_id = decode_guid(&mut cursor)?;
    let permission_tribe_id = decode_u8(&mut cursor)?;
    let unknown_tail = read_remaining(&mut cursor);

    Ok(json!({
        "codec_status": "decoded",
        "is_empty": false,
        "player_uid": player_uid,
        "instance_id": instance_id,
        "permission_tribe_id": permission_tribe_id,
        "unknown_tail_hex": to_hex(&unknown_tail),
        "original_bytes_hex": to_hex(bytes),
    }))
}

pub fn encode(value: &Value) -> Result<Vec<u8>, String> {
    passthrough_encode(value)
}
