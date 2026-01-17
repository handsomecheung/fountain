use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::fs;
use std::path::Path;

use crate::chunk::{split_into_chunks, split_into_chunks_with_size, DEFAULT_PAYLOAD_SIZE};
use crate::qr::{generate_qr_image, render_qr_to_terminal, save_qr_image};

pub struct EncodeResult {
    pub num_chunks: usize,
    pub output_files: Vec<String>,
}

pub struct TerminalQrData {
    pub filename: String,
    pub total: usize,
    pub qr_strings: Vec<String>,
}

pub fn encode_file(input_path: &Path, output_dir: &Path, chunk_size: Option<usize>) -> Result<EncodeResult> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?;

    fs::create_dir_all(output_dir)?;

    let chunks = if let Some(size) = chunk_size {
        split_into_chunks_with_size(&data, filename, size)?
    } else {
        split_into_chunks(&data, filename)?
    };
    
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
    })
}

pub fn encode_file_for_terminal(input_path: &Path) -> Result<TerminalQrData> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    // Use smaller chunks for terminal display (smaller QR codes, more of them)
    let chunks = split_into_chunks_with_size(&data, &filename, DEFAULT_PAYLOAD_SIZE)?;
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
    })
}