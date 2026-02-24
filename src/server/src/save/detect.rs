#[derive(Debug, Clone)]
pub struct SaveVariantInfo {
    pub has_cnk_prefix: bool,
    pub magic: Option<String>,
    pub save_type: Option<u8>,
    pub compression: String,
    pub uncompressed_size: Option<u32>,
    pub compressed_size: Option<u32>,
    pub payload_offset: usize,
    pub payload_len: usize,
}

pub fn detect_save_variant(bytes: &[u8]) -> SaveVariantInfo {
    if bytes.len() < 12 {
        return SaveVariantInfo {
            has_cnk_prefix: false,
            magic: None,
            save_type: None,
            compression: "unknown".to_string(),
            uncompressed_size: None,
            compressed_size: None,
            payload_offset: 0,
            payload_len: 0,
        };
    }

    let has_cnk_prefix = bytes.starts_with(b"CNK");
    let header_offset = if has_cnk_prefix { 12 } else { 0 };
    let payload_offset = if has_cnk_prefix { 24 } else { 12 };

    if bytes.len() < header_offset + 12 {
        return SaveVariantInfo {
            has_cnk_prefix,
            magic: None,
            save_type: None,
            compression: "unknown".to_string(),
            uncompressed_size: None,
            compressed_size: None,
            payload_offset,
            payload_len: 0,
        };
    }

    let uncompressed_size = read_u32_le(bytes, header_offset);
    let compressed_size = read_u32_le(bytes, header_offset + 4);
    let magic_bytes = &bytes[header_offset + 8..header_offset + 11];
    let save_type = bytes[header_offset + 11];
    let magic = Some(String::from_utf8_lossy(magic_bytes).to_string());

    let compression = if magic_bytes == b"PlZ" && save_type == 0x32 {
        "zlib"
    } else if magic_bytes == b"PlM" && save_type == 0x31 {
        "oodle"
    } else {
        "unknown"
    };

    let available_payload = bytes.len().saturating_sub(payload_offset);
    let requested_payload = compressed_size as usize;
    let payload_len = if requested_payload == 0 {
        available_payload
    } else {
        requested_payload.min(available_payload)
    };

    SaveVariantInfo {
        has_cnk_prefix,
        magic,
        save_type: Some(save_type),
        compression: compression.to_string(),
        uncompressed_size: Some(uncompressed_size),
        compressed_size: Some(compressed_size),
        payload_offset,
        payload_len,
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
