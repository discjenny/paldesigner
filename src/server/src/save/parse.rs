use crate::save::detect::SaveVariantInfo;
use flate2::read::ZlibDecoder;
use oozextract::Extractor;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct GvasInspectResult {
    pub decode_status: String,
    pub decode_error: Option<String>,
    pub decompressed_size: Option<u64>,
    pub gvas_magic: Option<String>,
}

pub fn inspect_gvas(bytes: &[u8], variant: &SaveVariantInfo) -> GvasInspectResult {
    match decode_to_gvas(bytes, variant) {
        Ok(decompressed) => {
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
        Err(error) if error == "decode_not_attempted" => GvasInspectResult {
            decode_status: "not_attempted".to_string(),
            decode_error: None,
            decompressed_size: None,
            gvas_magic: None,
        },
        Err(error) => GvasInspectResult {
            decode_status: "error".to_string(),
            decode_error: Some(error),
            decompressed_size: None,
            gvas_magic: None,
        },
    }
}

pub fn decode_to_gvas(bytes: &[u8], variant: &SaveVariantInfo) -> Result<Vec<u8>, String> {
    let payload = payload_slice(bytes, variant)?;

    match variant.compression.as_str() {
        "zlib" => decode_plz(payload, variant),
        "oodle" => decode_plm(payload, variant),
        _ => Err("decode_not_attempted".to_string()),
    }
}

fn decode_plz(payload: &[u8], variant: &SaveVariantInfo) -> Result<Vec<u8>, String> {
    let first_pass =
        zlib_decompress(payload).map_err(|error| format!("zlib decode failed: {}", error))?;
    let decoded = if variant.save_type == Some(0x32) {
        zlib_decompress(&first_pass)
            .map_err(|error| format!("zlib second-pass decode failed: {}", error))?
    } else {
        first_pass
    };

    if let Some(expected_size) = variant.uncompressed_size {
        let decoded_len = decoded.len() as u32;
        if decoded_len != expected_size {
            return Err(format!(
                "decoded size mismatch: expected {} bytes, got {} bytes",
                expected_size, decoded_len
            ));
        }
    }

    Ok(decoded)
}

fn decode_plm(payload: &[u8], variant: &SaveVariantInfo) -> Result<Vec<u8>, String> {
    let Some(expected_size) = variant.uncompressed_size else {
        return Err("oodle decode requires uncompressed_size from save header".to_string());
    };

    let mut output = vec![0u8; expected_size as usize];
    let mut extractor = Extractor::new();
    let bytes_written = extractor
        .read_from_slice(payload, output.as_mut_slice())
        .map_err(|error| format!("oodle decode failed: {}", error))?;

    if bytes_written != output.len() {
        return Err(format!(
            "oodle decoded byte count mismatch: expected {} bytes, got {} bytes",
            output.len(),
            bytes_written
        ));
    }

    Ok(output)
}

fn payload_slice<'a>(bytes: &'a [u8], variant: &SaveVariantInfo) -> Result<&'a [u8], String> {
    let payload_start = variant.payload_offset;
    let payload_end = payload_start.saturating_add(variant.payload_len);
    if payload_end > bytes.len() || payload_start >= payload_end {
        return Err("payload boundaries are invalid".to_string());
    }

    Ok(&bytes[payload_start..payload_end])
}

fn zlib_decompress(payload: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(payload);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

fn hex_magic(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02X}", byte)).collect()
}
