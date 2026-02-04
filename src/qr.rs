use anyhow::{anyhow, Result};

#[cfg(feature = "encode")]
use image::{Rgb, RgbImage};

#[cfg(any(feature = "decode", feature = "wasm"))]
use image::{DynamicImage, GrayImage};

#[cfg(feature = "encode")]
use qrcode::{Color, EcLevel, QrCode, Version};

#[cfg(any(feature = "decode", feature = "wasm"))]
use rqrr::PreparedImage;

#[cfg(any(feature = "encode", feature = "decode"))]
use std::path::Path;

#[cfg(feature = "encode")]
pub fn generate_qr_image(
    data: &[u8],
    specific_version: Option<Version>,
    pixel_scale: u32,
) -> Result<(RgbImage, Version)> {
    let code = if let Some(v) = specific_version {
        QrCode::with_version(data, v, EcLevel::M)
            .map_err(|e| anyhow!("Failed to create QR code with specific version: {}", e))?
    } else {
        QrCode::with_error_correction_level(data, EcLevel::M)
            .map_err(|e| anyhow!("Failed to create QR code: {}", e))?
    };

    let version = code.version();

    let image = code
        .render::<Rgb<u8>>()
        .min_dimensions(200, 200)
        .quiet_zone(true)
        .module_dimensions(pixel_scale, pixel_scale)
        .build();

    Ok((image, version))
}

#[cfg(feature = "encode")]
pub fn save_qr_image(image: &RgbImage, path: &Path) -> Result<()> {
    image.save(path)?;
    Ok(())
}

#[cfg(feature = "decode")]
pub fn decode_qr_image(path: &Path) -> Result<Vec<u8>> {
    let img = image::open(path)?;
    decode_qr_from_dynamic_image(&img)
}

#[cfg(any(feature = "decode", feature = "wasm"))]
pub fn decode_qr_from_dynamic_image(img: &DynamicImage) -> Result<Vec<u8>> {
    let gray = img.to_luma8();
    decode_qr_from_gray(&gray)
}

#[cfg(any(feature = "decode", feature = "wasm"))]
pub fn decode_qr_from_gray(gray: &GrayImage) -> Result<Vec<u8>> {
    let mut prepared = PreparedImage::prepare(gray.clone());
    let grids = prepared.detect_grids();

    if grids.is_empty() {
        return Err(anyhow!("No QR code found in image"));
    }

    let (_, content) = grids[0]
        .decode()
        .map_err(|e| anyhow!("Failed to decode QR code: {:?}", e))?;

    Ok(content.into_bytes())
}

#[cfg(feature = "encode")]
pub fn render_qr_to_terminal(data: &[u8]) -> Result<String> {
    use terminal_size::{terminal_size, Height, Width};

    let code = QrCode::with_error_correction_level(data, EcLevel::M)
        .map_err(|e| anyhow!("Failed to create QR code: {}", e))?;

    let qr_size = code.width();
    let colors = code.to_colors();

    let (term_width, term_height) = terminal_size()
        .map(|(Width(w), Height(h))| {
            if w < 40 || h < 30 {
                (120, 60)
            } else {
                (w as usize, h as usize)
            }
        })
        .unwrap_or((120, 60));

    let qr_with_quiet = qr_size + 4; // Add quiet zone

    // Fixed scale=1: each QR module = 1 char wide, uses half-blocks for height
    // This gives the most compact and square appearance
    let scale: usize = 1;

    let display_width = qr_with_quiet * scale;
    let display_height = (qr_with_quiet + 1) / 2 * scale;

    // Center padding
    let pad_left = term_width.saturating_sub(display_width) / 2;
    let pad_top = term_height.saturating_sub(display_height + 8) / 2;

    let mut result = String::new();
    let left_pad: String = " ".repeat(pad_left);

    // Top padding
    for _ in 0..pad_top {
        result.push('\n');
    }

    // Helper to check if a position is dark
    let is_dark = |row: usize, col: usize| -> bool {
        if row >= 2 && row < qr_size + 2 && col >= 2 && col < qr_size + 2 {
            let qr_y = row - 2;
            let qr_x = col - 2;
            colors[qr_y * qr_size + qr_x] == Color::Dark
        } else {
            false // Quiet zone is white
        }
    };

    // Render using half-block characters
    // Process 2 QR rows at a time, each becomes 1 terminal row (with scale repetition)
    for qr_row_pair in 0..((qr_with_quiet + 1) / 2) {
        let top_row = qr_row_pair * 2;
        let bottom_row = top_row + 1;

        // Repeat each output row 'scale' times for vertical scaling
        for _ in 0..scale {
            result.push_str(&left_pad);

            for qr_col in 0..qr_with_quiet {
                let top_dark = is_dark(top_row, qr_col);
                let bottom_dark = if bottom_row < qr_with_quiet {
                    is_dark(bottom_row, qr_col)
                } else {
                    false
                };

                let ch = match (top_dark, bottom_dark) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                };

                // Repeat char 'scale' times for horizontal scaling
                for _ in 0..scale {
                    result.push(ch);
                }
            }
            result.push('\n');
        }
    }

    Ok(result)
}

#[cfg(feature = "encode")]
pub fn fits_in_terminal(data: &[u8]) -> Result<bool> {
    use terminal_size::{terminal_size, Height, Width};

    let code = QrCode::with_error_correction_level(data, EcLevel::M)
        .map_err(|e| anyhow!("Failed to create QR code: {}", e))?;

    let qr_size = code.width();
    let qr_with_quiet = qr_size + 4; // Add quiet zone

    let scale: usize = 1;
    let display_width = qr_with_quiet * scale;
    let display_height = (qr_with_quiet + 1) / 2 * scale;

    let (term_width, term_height) = terminal_size()
        .map(|(Width(w), Height(h))| {
            if w < 40 || h < 30 {
                (120, 60)
            } else {
                (w as usize, h as usize)
            }
        })
        .unwrap_or((120, 60));

    // Check if it fits (allow 6 lines for header/footer/spacing)
    if display_width > term_width || display_height + 6 > term_height {
        Ok(false)
    } else {
        Ok(true)
    }
}

#[cfg(all(test, feature = "encode", feature = "decode"))]
mod tests {
    use super::*;

    #[test]
    fn test_qr_generation() {
        let data = b"Hello, World!";
        let (image, _) = generate_qr_image(data, None, 4).unwrap();
        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }

    #[test]
    fn test_qr_roundtrip() {
        let data = b"Test data for QR code roundtrip";
        let (image, _) = generate_qr_image(data, None, 4).unwrap();

        // Convert to grayscale for decoding
        let gray: GrayImage = image::DynamicImage::ImageRgb8(image).to_luma8();

        let decoded = decode_qr_from_gray(&gray).unwrap();
        assert_eq!(decoded, data);
    }
}
