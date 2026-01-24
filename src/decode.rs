use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage};
use opencv::{
    core::Mat,
    imgproc,
    objdetect::QRCodeDetector,
    prelude::*,
    videoio::{self, VideoCapture},
};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use crate::chunk::{merge_chunks, Chunk};
use crate::qr::{decode_qr_from_dynamic_image, decode_qr_image};

pub struct DecodeResult {
    pub original_filename: String,
    pub output_path: String,
    pub num_chunks: usize,
}

pub fn decode_from_gif(input_file: &Path, output_path: Option<&Path>) -> Result<DecodeResult> {
    let file = File::open(input_file)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    let frames = decoder.into_frames();

    println!("Decoding QR codes from GIF: {}", input_file.display());

    let mut chunks = HashMap::new();
    let mut frame_count = 0;
    let mut expected_total = None;

    for (i, frame_result) in frames.enumerate() {
        let frame = frame_result?;
        frame_count += 1;

        let buffer = frame.buffer();
        let dynamic_image = DynamicImage::ImageRgba8(buffer.clone());

        if let Ok(qr_bytes) = decode_qr_from_dynamic_image(&dynamic_image) {
            let qr_string = String::from_utf8_lossy(&qr_bytes).to_string();
            if let Ok(chunk_bytes) = BASE64.decode(&qr_string) {
                if let Ok(chunk) = Chunk::from_bytes(&chunk_bytes) {
                    if expected_total.is_none() {
                        expected_total = Some(chunk.header.total as usize);
                    }

                    if !chunks.contains_key(&chunk.header.index) {
                        println!(
                            "Found chunk {}/{} in frame {}",
                            chunk.header.index + 1,
                            chunk.header.total,
                            i + 1,
                        );
                        chunks.insert(chunk.header.index, chunk);
                    }
                }
            }
        }

        if let Some(total) = expected_total {
            if chunks.len() == total {
                println!("Collected all {} chunk(s). Stopping early.", total);
                break;
            }
        }
    }

    if chunks.is_empty() {
        return Err(anyhow!("No QR codes found in GIF"));
    }

    let total_chunks_in_file = chunks.values().next().map(|c| c.header.total).unwrap_or(0);

    println!(
        "Found {} unique QR code(s) out of a total of {} in {} frames",
        chunks.len(),
        total_chunks_in_file,
        frame_count
    );

    if chunks.len() != total_chunks_in_file as usize {
        println!(
            "Warning: Mismatch in found chunks ({}) and expected total ({})",
            chunks.len(),
            total_chunks_in_file
        );
    }

    let mut sorted_chunks: Vec<Chunk> = chunks.into_values().collect();
    sorted_chunks.sort_by_key(|c| c.header.index);

    let num_chunks = sorted_chunks.len();
    let (original_filename, data) = merge_chunks(sorted_chunks)?;

    let final_output_path = match output_path {
        Some(p) => p.to_path_buf(),
        None => Path::new(".").join(&original_filename),
    };

    fs::write(&final_output_path, &data)?;

    Ok(DecodeResult {
        original_filename,
        output_path: final_output_path.to_string_lossy().to_string(),
        num_chunks,
    })
}

pub fn decode_from_images(input_dir: &Path, output_path: Option<&Path>) -> Result<DecodeResult> {
    let png_files: Vec<_> = fs::read_dir(input_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext.to_ascii_lowercase() == "png")
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect();

    if png_files.is_empty() {
        return Err(anyhow!("No PNG files found in directory"));
    }

    println!("Found {} QR code image(s)", png_files.len());

    let mut chunks = HashMap::new();
    let mut expected_total = None;

    for (i, png_path) in png_files.iter().enumerate() {
        println!(
            "  Decoding {}/{}: {}",
            i + 1,
            png_files.len(),
            png_path.file_name().unwrap_or_default().to_string_lossy()
        );

        let qr_data = decode_qr_image(png_path)?;

        let qr_string = String::from_utf8(qr_data)?;
        let chunk_bytes = BASE64
            .decode(&qr_string)
            .map_err(|e| anyhow!("Failed to decode base64: {}", e))?;

        let chunk = Chunk::from_bytes(&chunk_bytes)?;

        if expected_total.is_none() {
            expected_total = Some(chunk.header.total as usize);
        }

        chunks.insert(chunk.header.index, chunk);

        if let Some(total) = expected_total {
            if chunks.len() == total {
                println!("Collected all {} chunk(s). Stopping early.", total);
                break;
            }
        }
    }

    let mut sorted_chunks: Vec<Chunk> = chunks.into_values().collect();
    sorted_chunks.sort_by_key(|c| c.header.index);

    let num_chunks = sorted_chunks.len();
    let (original_filename, data) = merge_chunks(sorted_chunks)?;

    let final_output_path = match output_path {
        Some(p) => p.to_path_buf(),
        None => {
            let parent = input_dir.parent().unwrap_or(Path::new("."));
            parent.join(&original_filename)
        }
    };

    fs::write(&final_output_path, &data)?;

    Ok(DecodeResult {
        original_filename,
        output_path: final_output_path.to_string_lossy().to_string(),
        num_chunks,
    })
}

pub fn decode_from_video(input_file: &Path, output_path: Option<&Path>) -> Result<DecodeResult> {
    let mut cam = VideoCapture::from_file(&input_file.to_string_lossy(), videoio::CAP_ANY)?;
    if !cam.is_opened()? {
        return Err(anyhow!(
            "Failed to open video file: {}",
            input_file.display()
        ));
    }

    let frame_count = cam.get(videoio::CAP_PROP_FRAME_COUNT)? as u64;
    println!("Video has {} frames. Starting scan...", frame_count);

    let mut chunks = HashMap::new();
    let mut frame = Mat::default();
    let mut gray_frame = Mat::default();
    let mut points = Mat::default();
    let mut straight_code = Mat::default();
    let detector = QRCodeDetector::default()?;
    let mut expected_total = None;

    for i in 0..frame_count {
        if !cam.read(&mut frame)? {
            break;
        }

        // OpenCV's QRCodeDetector works best on grayscale images too,
        // though it can handle color. Converting to gray is safer.
        imgproc::cvt_color(
            &frame,
            &mut gray_frame,
            imgproc::COLOR_BGR2GRAY,
            0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;

        let mut qr_bytes =
            detector.detect_and_decode(&gray_frame, &mut points, &mut straight_code)?;

        // If detection fails, try inverting the image (handle dark-on-light vs light-on-dark)
        if qr_bytes.is_empty() {
            let mut inverted_frame = Mat::default();
            opencv::core::bitwise_not(&gray_frame, &mut inverted_frame, &opencv::core::no_array())?;
            qr_bytes =
                detector.detect_and_decode(&inverted_frame, &mut points, &mut straight_code)?;
        }

        if !qr_bytes.is_empty() {
            let qr_string = String::from_utf8_lossy(&qr_bytes).to_string();
            if let Ok(chunk_bytes) = BASE64.decode(&qr_string) {
                if let Ok(chunk) = Chunk::from_bytes(&chunk_bytes) {
                    if expected_total.is_none() {
                        expected_total = Some(chunk.header.total as usize);
                    }

                    if !chunks.contains_key(&chunk.header.index) {
                        println!(
                            "Found chunk {}/{} in frame {}",
                            chunk.header.index + 1,
                            chunk.header.total,
                            i + 1,
                        );
                        chunks.insert(chunk.header.index, chunk);
                    }
                }
            }
        }

        if let Some(total) = expected_total {
            if chunks.len() == total {
                println!("Collected all {} chunk(s). Stopping early.", total);
                break;
            }
        }
    }

    if chunks.is_empty() {
        return Err(anyhow!("No QR codes found in video"));
    }

    let total_chunks_in_file = chunks.values().next().map(|c| c.header.total).unwrap_or(0);

    println!(
        "Found {} unique QR code(s) out of a total of {}",
        chunks.len(),
        total_chunks_in_file
    );

    if chunks.len() != total_chunks_in_file as usize {
        println!(
            "Warning: Mismatch in found chunks ({}) and expected total ({})",
            chunks.len(),
            total_chunks_in_file
        );
    }

    let mut sorted_chunks: Vec<Chunk> = chunks.into_values().collect();
    sorted_chunks.sort_by_key(|c| c.header.index);

    let num_chunks = sorted_chunks.len();
    let (original_filename, data) = merge_chunks(sorted_chunks)?;

    let final_output_path = match output_path {
        Some(p) => p.to_path_buf(),
        None => Path::new(".").join(&original_filename),
    };

    fs::write(&final_output_path, &data)?;

    Ok(DecodeResult {
        original_filename,
        output_path: final_output_path.to_string_lossy().to_string(),
        num_chunks,
    })
}
