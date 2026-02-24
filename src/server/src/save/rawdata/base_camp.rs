use crate::save::rawdata::{
    decode_f32, decode_fstring, decode_guid, decode_u8, passthrough_encode, read_bytes,
    read_remaining, to_hex,
};
use serde_json::{Value, json};
use std::io::Cursor;

pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    let mut cursor = Cursor::new(bytes);
    let id = decode_guid(&mut cursor)?;
    let name = decode_fstring(&mut cursor)?;
    let state = decode_u8(&mut cursor)?;
    let transform = read_bytes(&mut cursor, 80)?;
    let area_range = decode_f32(&mut cursor)?;
    let group_id_belong_to = decode_guid(&mut cursor)?;
    let fast_travel_local_transform = read_bytes(&mut cursor, 80)?;
    let owner_map_object_instance_id = decode_guid(&mut cursor)?;
    let unknown_tail = read_remaining(&mut cursor);

    Ok(json!({
        "codec_status": "decoded",
        "id": id,
        "name": name,
        "state": state,
        "transform_hex": to_hex(&transform),
        "area_range": area_range,
        "group_id_belong_to": group_id_belong_to,
        "fast_travel_local_transform_hex": to_hex(&fast_travel_local_transform),
        "owner_map_object_instance_id": owner_map_object_instance_id,
        "unknown_tail_hex": to_hex(&unknown_tail),
        "original_bytes_hex": to_hex(bytes),
    }))
}

pub fn encode(value: &Value) -> Result<Vec<u8>, String> {
    passthrough_encode(value)
}
