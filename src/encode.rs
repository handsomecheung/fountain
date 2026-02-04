use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, RgbaImage};
use qrcode::Version;
use raptorq::Encoder;
use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::chunk::{
    compress, pack_data, split_compressed_into_chunks, split_into_chunks, Chunk, ChunkHeader,
    DEFAULT_PAYLOAD_SIZE, V1_HEADER_SIZE,
};
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
fn prepare_chunks(
    input_path: &Path,
    chunk_size: Option<usize>,
    pixel_scale: u32,
) -> Result<(Vec<Chunk>, usize, String)> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    let packed = pack_data(&data, &filename);

    let compressed = compress(&packed)?;

    let mut current_size = chunk_size.unwrap_or(crate::chunk::MAX_PAYLOAD_SIZE);

    loop {
        let mut chunks_iter = split_compressed_into_chunks(&compressed, current_size);

        // Get the first chunk to test if it fits
        if let Some(first_chunk) = chunks_iter.next() {
            let chunk_bytes = first_chunk.to_bytes()?;
            let encoded = BASE64.encode(&chunk_bytes);

            if generate_qr_image(encoded.as_bytes(), None, pixel_scale).is_ok() {
                // First chunk fits, assume the rest fit too. Collect the rest of the chunks.
                let mut chunks = vec![first_chunk];
                chunks.extend(chunks_iter);
                return Ok((chunks, current_size, filename));
            }
        }

        if current_size > 100 {
            current_size -= 50;
        } else {
            return Err(anyhow!(
                "Failed to generate QR codes: data too long even at minimum payload size."
            ));
        }
    }
}

fn prepare_raptorq_chunks(
    input_path: &Path,
    chunk_size: Option<usize>,
    pixel_scale: u32,
    redundancy_factor: f64,
) -> Result<(Vec<Chunk>, usize, String)> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    let packed = pack_data(&data, &filename);
    let compressed = compress(&packed)?;

    // Start with requested size or max, but we might need to reduce it if QR generation fails
    let mut current_size = chunk_size.unwrap_or(crate::chunk::MAX_PAYLOAD_SIZE);

    loop {
        // Ensure packet size is even for RaptorQ
        let packet_size = (current_size.saturating_sub(V1_HEADER_SIZE)) as u16;
        let packet_size = packet_size - (packet_size % 2); 
        
        if packet_size < 4 {
             return Err(anyhow!("Payload size too small for RaptorQ"));
        }

        let encoder = Encoder::with_defaults(&compressed, packet_size);
        
        // Generate one packet to test QR fit
        let test_packets = encoder.get_encoded_packets(1);
        if let Some(first_packet) = test_packets.first() {
             let chunk = Chunk {
                header: ChunkHeader {
                    version: 1,
                    total: compressed.len() as u32,
                    index: 0, 
                    packet_size,
                },
                data: first_packet.serialize(),
            };
            
            let chunk_bytes = chunk.to_bytes()?;
            let encoded = BASE64.encode(&chunk_bytes);
            
             if generate_qr_image(encoded.as_bytes(), None, pixel_scale).is_ok() {
                 // Fits. Generate all packets.
                 let source_packets = (compressed.len() as f64 / packet_size as f64).ceil() as u32;
                 let total_packets = (source_packets as f64 * redundancy_factor).ceil() as u32;
                 let total_packets = total_packets.max(source_packets + 2); 
                 
                 let packets_data = encoder.get_encoded_packets(total_packets);
                 let mut chunks = Vec::new();
                 
                 for (i, packet) in packets_data.into_iter().enumerate() {
                    chunks.push(Chunk {
                        header: ChunkHeader {
                            version: 1,
                            total: compressed.len() as u32,
                            index: i as u32,
                            packet_size,
                        },
                        data: packet.serialize(),
                    });
                }
                
                return Ok((chunks, current_size, filename));
             }
        }
        
         if current_size > 100 {
            current_size -= 50;
        } else {
            return Err(anyhow!(
                "Failed to generate QR codes: data too long even at minimum payload size."
            ));
        }
    }
}

pub fn encode_file_to_images(
    input_path: &Path,
    output_dir: &Path,
    chunk_size: Option<usize>,
    pixel_scale: u32,
    use_raptorq: bool,
) -> Result<EncodeResult> {
    fs::create_dir_all(output_dir)?;

    let (chunks, effective_size, filename) = if use_raptorq {
        prepare_raptorq_chunks(input_path, chunk_size, pixel_scale, 1.5)?
    } else {
        prepare_chunks(input_path, chunk_size, pixel_scale)?
    };

    let num_chunks = chunks.len();
    let mut output_files = Vec::new();

    let mut fixed_version: Option<Version> = None;

    for chunk in &chunks {
        let chunk_bytes = chunk.to_bytes()?;

        let encoded = BASE64.encode(&chunk_bytes);

        let (qr_image, version) =
            generate_qr_image(encoded.as_bytes(), fixed_version, pixel_scale)?;

        // Capture the version of the first chunk (which is typically the largest/full)
        // and use it for all subsequent chunks to ensure consistent image dimensions.
        if fixed_version.is_none() {
            fixed_version = Some(version);
        }

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

pub fn encode_file_for_terminal_raptorq(
    input_path: &Path,
    chunk_size: Option<usize>,
) -> Result<TerminalQrData> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    let packed = pack_data(&data, &filename);
    let compressed = compress(&packed)?;

    let mut current_size = chunk_size.unwrap_or(DEFAULT_PAYLOAD_SIZE);

    loop {
        // Try to initialize encoder with this packet size
        let packet_size = (current_size - V1_HEADER_SIZE) as u16;
        let packet_size = packet_size - (packet_size % 2); // Ensure even

        let encoder = Encoder::with_defaults(&compressed, packet_size);

        // Generate one packet to test size
        let packets = encoder.get_encoded_packets(1);
        if let Some(first_packet) = packets.first() {
            let chunk = Chunk {
                header: ChunkHeader {
                    version: 1,
                    total: compressed.len() as u32,
                    index: 0, // Placeholder
                    packet_size,
                },
                data: first_packet.serialize(),
            };

            let chunk_bytes = chunk.to_bytes()?;
            let encoded = BASE64.encode(&chunk_bytes);

            if crate::qr::fits_in_terminal(encoded.as_bytes())? {
                // Fits! Generate a stream of packets.
                // For "infinite stream" simulation in a carousel, we generate a reasonable number
                // of packets (e.g., 1.5x - 2.0x the source packets) and let the carousel loop them.
                // RaptorQ packets are distinct enough that looping 1.5x is good.

                let source_packets = (compressed.len() as f64 / packet_size as f64).ceil() as u32;
                // Generate enough for 2.0x coverage to give good redundancy
                // We generate repair packets (which are sufficient for decoding)
                let repair_packets = source_packets * 2;

                let all_packets = encoder.get_encoded_packets(repair_packets);
                let total = all_packets.len();
                let mut qr_strings = Vec::new();

                for (i, packet) in all_packets.into_iter().enumerate() {
                    let chunk = Chunk {
                        header: ChunkHeader {
                            version: 1,
                            total: compressed.len() as u32,
                            index: i as u32, // ESI
                            packet_size,
                        },
                        data: packet.serialize(),
                    };

                    let chunk_bytes = chunk.to_bytes()?;
                    let encoded = BASE64.encode(&chunk_bytes);
                    let qr_string = render_qr_to_terminal(encoded.as_bytes())?;
                    qr_strings.push(qr_string);
                }

                return Ok(TerminalQrData {
                    filename,
                    total,
                    qr_strings,
                    effective_size: current_size,
                });
            }
        }

        if current_size > 50 {
            current_size -= 20;
        } else {
            return Err(anyhow!("Terminal too small to display QR codes even at minimum payload size. Please increase terminal size."));
        }
    }
}

pub fn encode_file_to_gif(
    input_path: &Path,
    output_gif: &Path,
    chunk_size: Option<usize>,
    interval_ms: u64,
    pixel_scale: u32,
    use_raptorq: bool,
) -> Result<EncodeResult> {
    let (chunks, effective_size, _filename) = if use_raptorq {
        prepare_raptorq_chunks(input_path, chunk_size, pixel_scale, 1.5)?
    } else {
        prepare_chunks(input_path, chunk_size, pixel_scale)?
    };

    let num_chunks = chunks.len();

    if let Some(parent) = output_gif.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::File::create(output_gif)?;
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    let should_print_progress = num_chunks > 10;

    let mut fixed_version: Option<Version> = None;

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_bytes = chunk.to_bytes()?;
        let encoded = BASE64.encode(&chunk_bytes);

        let (qr_image, version) =
            generate_qr_image(encoded.as_bytes(), fixed_version, pixel_scale)?;

        if fixed_version.is_none() {
            fixed_version = Some(version);
        }

        let rgba_image: RgbaImage = image::DynamicImage::ImageRgb8(qr_image).into_rgba8();

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

pub fn encode_data(
    data: &[u8],
    filename: &str,
    output_dir: &Path,
    pixel_scale: u32,
) -> Result<EncodeResult> {
    fs::create_dir_all(output_dir)?;

    let chunks = split_into_chunks(data, filename)?;
    let num_chunks = chunks.len();
    let mut output_files = Vec::new();

    let mut fixed_version: Option<Version> = None;

    for chunk in &chunks {
        let chunk_bytes = chunk.to_bytes()?;
        let encoded = BASE64.encode(&chunk_bytes);

        let (qr_image, version) =
            generate_qr_image(encoded.as_bytes(), fixed_version, pixel_scale)?;
        if fixed_version.is_none() {
            fixed_version = Some(version);
        }

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

pub fn encode_file_for_terminal(
    input_path: &Path,
    chunk_size: Option<usize>,
) -> Result<TerminalQrData> {
    let data = fs::read(input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    let packed = pack_data(&data, &filename);
    let compressed = compress(&packed)?;

    let mut current_size = chunk_size.unwrap_or(DEFAULT_PAYLOAD_SIZE);

    loop {
        let mut chunks_iter = split_compressed_into_chunks(&compressed, current_size);

        if let Some(first_chunk) = chunks_iter.next() {
            let chunk_bytes = first_chunk.to_bytes()?;
            let encoded = BASE64.encode(&chunk_bytes);

            if crate::qr::fits_in_terminal(encoded.as_bytes())? {
                // Fits! Generate all chunks
                let mut chunks = vec![first_chunk];
                chunks.extend(chunks_iter);

                let total = chunks.len();
                let mut qr_strings = Vec::new();

                for chunk in &chunks {
                    let chunk_bytes = chunk.to_bytes()?;
                    let encoded = BASE64.encode(&chunk_bytes);
                    let qr_string = render_qr_to_terminal(encoded.as_bytes())?;
                    qr_strings.push(qr_string);
                }

                return Ok(TerminalQrData {
                    filename,
                    total,
                    qr_strings,
                    effective_size: current_size,
                });
            }
        }

        if current_size > 50 {
            current_size -= 20;
        } else {
            return Err(anyhow!("Terminal too small to display QR codes even at minimum payload size. Please increase terminal size."));
        }
    }
}