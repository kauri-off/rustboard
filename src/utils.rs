use std::io::Cursor;

use sha2::{Digest, Sha256};

pub fn hash_ip(ip: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(ip.as_bytes());
    h.update(salt.as_bytes());
    hex::encode(&h.finalize()[..8])
}

/// Validates image dimensions and re-encodes to strip EXIF metadata.
/// GIFs are returned as-is to preserve animation frames.
pub fn process_image(
    bytes: &[u8],
    ext: &str,
    max_width: u32,
    max_height: u32,
) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes).map_err(|_| "Invalid image data".to_string())?;

    if img.width() > max_width || img.height() > max_height {
        return Err(format!(
            "Image dimensions too large. Max {}x{} pixels",
            max_width, max_height
        ));
    }

    // GIFs: return as-is to preserve animation
    if ext == "gif" {
        return Ok(bytes.to_vec());
    }

    // JPEG, PNG, WebP: re-encode through image crate to strip EXIF metadata
    let format = match ext {
        "jpg" | "jpeg" => image::ImageFormat::Jpeg,
        "png" => image::ImageFormat::Png,
        "webp" => image::ImageFormat::WebP,
        _ => return Err("Unsupported image format".to_string()),
    };

    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, format)
        .map_err(|e| format!("Failed to process image: {e}"))?;

    Ok(buf.into_inner())
}
