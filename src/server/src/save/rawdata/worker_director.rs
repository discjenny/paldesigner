use crate::save::rawdata::{
    decode_guid, decode_u8, passthrough_encode, read_bytes, read_remaining, to_hex,
};
use serde_json::{Value, json};
use std::io::Cursor;

pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    let mut cursor = Cursor::new(bytes);
    let id = decode_guid(&mut cursor)?;
    let spawn_transform = read_bytes(&mut cursor, 80)?;
    let current_order_type = decode_u8(&mut cursor)?;
    let current_battle_type = decode_u8(&mut cursor)?;
    let container_id = decode_guid(&mut cursor)?;
    let unknown_tail = read_remaining(&mut cursor);

    Ok(json!({
        "codec_status": "decoded",
        "id": id,
        "spawn_transform_hex": to_hex(&spawn_transform),
        "current_order_type": current_order_type,
        "current_battle_type": current_battle_type,
        "container_id": container_id,
        "unknown_tail_hex": to_hex(&unknown_tail),
        "original_bytes_hex": to_hex(bytes),
    }))
}

pub fn encode(value: &Value) -> Result<Vec<u8>, String> {
    passthrough_encode(value)
}
