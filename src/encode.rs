use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::fs;
use std::path::Path;
use image::{Frame, Delay, RgbaImage};
use image::codecs::gif::GifEncoder;
use std::time::Duration;

use crate::chunk::{split_into_chunks, split_into_chunks_with_size, DEFAULT_PAYLOAD_SIZE, Chunk};
use crate::qr::{generate_qr_image, render_qr_to_terminal, save_qr_image};

pub struct EncodeResult {
    pub num_chunks: usize,
    pub output_files: Vec<String>,
    pub effective_size: usize,
}

pub struct TerminalQrData {
    pub filename: String,
    pub total: usize,
    pub qr_strings: Vec<String>,
    pub effective_size: usize,
}

/// Helper function to split data into chunks and ensure they fit into QR codes.
/// Returns the chunks, the effective payload size used, and the filename string.
fn prepare_chunks(input_path: &Path, chunk_size: Option<usize>) -> Result<(Vec<Chunk>, usize, String)> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    let mut current_size = chunk_size.unwrap_or(crate::chunk::MAX_PAYLOAD_SIZE);
    let mut chunks;

    loop {
        chunks = split_into_chunks_with_size(&data, &filename, current_size)?;
        
        // Test first chunk to see if it generates a valid QR code
        if let Some(first_chunk) = chunks.first() {
            let chunk_bytes = first_chunk.to_bytes()?;
            let encoded = BASE64.encode(&chunk_bytes);
            
            if generate_qr_image(encoded.as_bytes()).is_ok() {
                break;
            }
        } else {
            break; // No data, nothing to do
        }

        if current_size > 100 {
            current_size -= 50;
        } else {
             return Err(anyhow!("Failed to generate QR codes: data too long even at minimum payload size."));
        }
    }
    
    Ok((chunks, current_size, filename))
}

pub fn encode_file(input_path: &Path, output_dir: &Path, chunk_size: Option<usize>) -> Result<EncodeResult> {
    fs::create_dir_all(output_dir)?;

    let (chunks, effective_size, filename) = prepare_chunks(input_path, chunk_size)?;
    
    let num_chunks = chunks.len();
    let mut output_files = Vec::new();

    for chunk in &chunks {
        let chunk_bytes = chunk.to_bytes()?;

        let encoded = BASE64.encode(&chunk_bytes);

        // We already tested generation, but handle errors just in case
        let qr_image = generate_qr_image(encoded.as_bytes())?;

        let output_filename = format!(
            "{}_{:04}.png",
            filename.replace('.', "_"),
            chunk.header.index + 1
        );
        let output_path = output_dir.join(&output_filename);
        save_qr_image(&qr_image, &output_path)?;

        println!(
            "  Generated QR code {}/{}: {}",
            chunk.header.index + 1,
            num_chunks,
            &output_filename
        );

        output_files.push(output_filename);
    }

    Ok(EncodeResult {
        num_chunks,
        output_files,
        effective_size,
    })
}

pub fn encode_file_to_gif(input_path: &Path, output_gif: &Path, chunk_size: Option<usize>, interval_ms: u64) -> Result<EncodeResult> {
    let (chunks, effective_size, _filename) = prepare_chunks(input_path, chunk_size)?;
    let num_chunks = chunks.len();

    // Ensure parent directory exists
    if let Some(parent) = output_gif.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::File::create(output_gif)?;
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    // Only print progress every few frames if there are many
    let should_print_progress = num_chunks > 10;

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_bytes = chunk.to_bytes()?;
        let encoded = BASE64.encode(&chunk_bytes);
        let qr_image = generate_qr_image(encoded.as_bytes())?;

        // Convert RgbImage to RgbaImage for GIF
        let rgba_image: RgbaImage = image::DynamicImage::ImageRgb8(qr_image).into_rgba8();
        
        // Create frame with delay
        let delay = Delay::from_saturating_duration(Duration::from_millis(interval_ms));
        let frame = Frame::from_parts(rgba_image, 0, 0, delay);

        encoder.encode_frame(frame)?;

        if should_print_progress {
            if (i + 1) % 10 == 0 || i + 1 == num_chunks {
                 println!("  Processed frame {}/{}", i + 1, num_chunks);
            }
        } else {
             println!("  Processed frame {}/{}", i + 1, num_chunks);
        }
    }

    Ok(EncodeResult {
        num_chunks,
        output_files: vec![output_gif.to_string_lossy().to_string()],
        effective_size,
    })
}

pub fn encode_data(data: &[u8], filename: &str, output_dir: &Path) -> Result<EncodeResult> {
    fs::create_dir_all(output_dir)?;

    let chunks = split_into_chunks(data, filename)?;
    let num_chunks = chunks.len();
    let mut output_files = Vec::new();

    for chunk in &chunks {
        let chunk_bytes = chunk.to_bytes()?;
        let encoded = BASE64.encode(&chunk_bytes);
        let qr_image = generate_qr_image(encoded.as_bytes())?;

        let output_filename = format!(
            "{}_{:04}.png",
            filename.replace('.', "_"),
            chunk.header.index + 1
        );
        let output_path = output_dir.join(&output_filename);
        save_qr_image(&qr_image, &output_path)?;
        output_files.push(output_filename);
    }

    Ok(EncodeResult {
        num_chunks,
        output_files,
        effective_size: crate::chunk::MAX_PAYLOAD_SIZE,
    })
}

pub fn encode_file_for_terminal(input_path: &Path, chunk_size: Option<usize>) -> Result<TerminalQrData> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    // Use smaller chunks for terminal display (smaller QR codes, more of them)
    // Default to DEFAULT_PAYLOAD_SIZE if not provided
    let mut current_size = chunk_size.unwrap_or(DEFAULT_PAYLOAD_SIZE);
    let mut chunks;

    // Automatic resizing loop
    loop {
        chunks = split_into_chunks_with_size(&data, &filename, current_size)?;
        
        // Check if the first chunk fits (usually representative)
        if let Some(first_chunk) = chunks.first() {
             let chunk_bytes = first_chunk.to_bytes()?;
             let encoded = BASE64.encode(&chunk_bytes);
             
             if crate::qr::fits_in_terminal(encoded.as_bytes())? {
                 break; // Fits!
             }
        } else {
             break; // No data
        }

        // Reduce size and retry
        if current_size > 50 {
            current_size -= 20;
        } else {
             return Err(anyhow!("Terminal too small to display QR codes even at minimum payload size. Please increase terminal size."));
        }
    }

    let total = chunks.len();
    let mut qr_strings = Vec::new();

    for chunk in &chunks {
        let chunk_bytes = chunk.to_bytes()?;
        let encoded = BASE64.encode(&chunk_bytes);
        let qr_string = render_qr_to_terminal(encoded.as_bytes())?;
        qr_strings.push(qr_string);
    }

    Ok(TerminalQrData {
        filename,
        total,
        qr_strings,
        effective_size: current_size,
    })
}