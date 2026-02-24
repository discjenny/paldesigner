use crate::save::detect::SaveVariantInfo;
use flate2::read::ZlibDecoder;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct GvasInspectResult {
    pub decode_status: String,
    pub decode_error: Option<String>,
    pub decompressed_size: Option<u64>,
    pub gvas_magic: Option<String>,
}

pub fn inspect_gvas(bytes: &[u8], variant: &SaveVariantInfo) -> GvasInspectResult {
    if variant.compression != "zlib" {
        return GvasInspectResult {
            decode_status: "not_attempted".to_string(),
            decode_error: None,
            decompressed_size: None,
            gvas_magic: None,
        };
    }

    let payload_start = variant.payload_offset;
    let payload_end = payload_start.saturating_add(variant.payload_len);
    if payload_end > bytes.len() || payload_start >= payload_end {
        return GvasInspectResult {
            decode_status: "error".to_string(),
            decode_error: Some("zlib payload boundaries are invalid".to_string()),
            decompressed_size: None,
            gvas_magic: None,
        };
    }

    let payload = &bytes[payload_start..payload_end];
    let mut decoder = ZlibDecoder::new(payload);
    let mut decompressed = Vec::new();

    if let Err(error) = decoder.read_to_end(&mut decompressed) {
        return GvasInspectResult {
            decode_status: "error".to_string(),
            decode_error: Some(format!("zlib decode failed: {}", error)),
            decompressed_size: None,
            gvas_magic: None,
        };
    }

    let gvas_magic = if decompressed.len() >= 4 {
        let first_four = &decompressed[0..4];
        if first_four == b"GVAS" {
            Some("GVAS".to_string())
        } else {
            Some(hex_magic(first_four))
        }
    } else {
        None
    };

    GvasInspectResult {
        decode_status: "ok".to_string(),
        decode_error: None,
        decompressed_size: Some(decompressed.len() as u64),
        gvas_magic,
    }
}

fn hex_magic(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02X}", byte)).collect()
}
