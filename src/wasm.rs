use crate::chunk::{decompress, merge_chunks, unpack_data, Chunk};
use crate::qr::decode_qr_from_gray;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::GrayImage;
use raptorq::{Decoder, EncodingPacket, ObjectTransmissionInformation};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum DecodeMode {
    Unknown,
    Standard, // Version 0
    RaptorQ,  // Version 1
}

#[wasm_bindgen]
pub struct QrStreamDecoder {
    chunks: HashMap<u32, Chunk>,
    total_chunks: Option<u32>,
    mode: DecodeMode,
    decoder_raptorq: Option<Decoder>,
    raptorq_transfer_length: Option<u64>,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct ScanResult {
    pub status: ScanStatus,
    pub progress_current: u32,
    pub progress_total: u32,
    filename: String,
    file_data: Vec<u8>,
}

#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq)]
pub enum ScanStatus {
    Scanning = 0,
    ChunkFound = 1,
    Complete = 2,
    Error = 3,
}

#[wasm_bindgen]
impl ScanResult {
    pub fn get_filename(&self) -> String {
        self.filename.clone()
    }

    pub fn get_file_data(&self) -> Vec<u8> {
        self.file_data.clone()
    }
}

#[wasm_bindgen]
impl QrStreamDecoder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> QrStreamDecoder {
        console_error_panic_hook::set_once();
        QrStreamDecoder {
            chunks: HashMap::new(),
            total_chunks: None,
            mode: DecodeMode::Unknown,
            decoder_raptorq: None,
            raptorq_transfer_length: None,
        }
    }

    pub fn scan_frame(&mut self, data: &[u8], width: u32, height: u32) -> ScanResult {
        if data.len() as u32 != width * height * 4 {
            return self.make_result(ScanStatus::Error, "Invalid data length".to_string(), vec![]);
        }

        let mut gray_pixels = Vec::with_capacity((width * height) as usize);
        for i in 0..(width * height) as usize {
            let offset = i * 4;
            let r = data[offset] as u32;
            let g = data[offset + 1] as u32;
            let b = data[offset + 2] as u32;
            let luma = (r * 299 + g * 587 + b * 114) / 1000;
            gray_pixels.push(luma as u8);
        }

        let mut gray_image = match GrayImage::from_raw(width, height, gray_pixels) {
            Some(img) => img,
            None => {
                return self.make_result(
                    ScanStatus::Error,
                    "Failed to create image".to_string(),
                    vec![],
                )
            }
        };

        // Try normal decode
        if let Some(result) = self.try_decode(&gray_image) {
            return result;
        }

        // Try inverted decode (for dark mode / inverted QR codes)
        for pixel in gray_image.iter_mut() {
            *pixel = 255 - *pixel;
        }

        if let Some(result) = self.try_decode(&gray_image) {
            return result;
        }

        self.current_status(ScanStatus::Scanning)
    }

    fn try_decode(&mut self, img: &GrayImage) -> Option<ScanResult> {
        if let Ok(qr_bytes) = decode_qr_from_gray(img) {
            let qr_string = String::from_utf8_lossy(&qr_bytes).to_string();
            if let Ok(chunk_bytes) = BASE64.decode(qr_string.trim()) {
                if let Ok(chunk) = Chunk::from_bytes(&chunk_bytes) {
                    return Some(self.process_chunk(chunk));
                }
            }
        }
        None
    }

    fn process_chunk(&mut self, chunk: Chunk) -> ScanResult {
        // Detect mode on first chunk
        if self.mode == DecodeMode::Unknown {
            self.mode = if chunk.header.version == 1 {
                DecodeMode::RaptorQ
            } else {
                DecodeMode::Standard
            };
        }

        match self.mode {
            DecodeMode::Standard => {
                if chunk.header.version != 0 {
                    return self.current_status(ScanStatus::Scanning);
                }

                let chunk_total = chunk.header.total as u32;
                let chunk_index = chunk.header.index as u32;

                if self.total_chunks.is_none() {
                    self.total_chunks = Some(chunk_total);
                }

                if let Some(total) = self.total_chunks {
                    if total != chunk_total {
                        return self.current_status(ScanStatus::Scanning);
                    }
                }

                if !self.chunks.contains_key(&chunk_index) {
                    self.chunks.insert(chunk_index, chunk);

                    if let Some(total) = self.total_chunks {
                        if self.chunks.len() as u32 == total {
                            let mut sorted_chunks: Vec<Chunk> =
                                self.chunks.values().cloned().collect();
                            sorted_chunks.sort_by_key(|c| c.header.index);

                            match merge_chunks(sorted_chunks) {
                                Ok((filename, data)) => {
                                    return self.make_result(ScanStatus::Complete, filename, data);
                                }
                                Err(_) => {
                                    return self.make_result(
                                        ScanStatus::Error,
                                        "Merge failed".to_string(),
                                        vec![],
                                    );
                                }
                            }
                        }
                    }
                    return self.current_status(ScanStatus::ChunkFound);
                }
            }
            DecodeMode::RaptorQ => {
                if chunk.header.version != 1 {
                    return self.current_status(ScanStatus::Scanning);
                }

                if self.decoder_raptorq.is_none() {
                    let transfer_len = chunk.header.total as u64;
                    let packet_size = chunk.header.packet_size;
                    self.raptorq_transfer_length = Some(transfer_len);

                    let config =
                        ObjectTransmissionInformation::with_defaults(transfer_len, packet_size);
                    self.decoder_raptorq = Some(Decoder::new(config));

                    // Estimate total packets needed (K) for progress bar
                    // Using ceiling division
                    let source_packets = (transfer_len as f64 / packet_size as f64).ceil() as u32;
                    self.total_chunks = Some(source_packets);
                }

                if !self.chunks.contains_key(&chunk.header.index) {
                    self.chunks.insert(chunk.header.index, chunk.clone());

                    if let Some(dec) = &mut self.decoder_raptorq {
                        let packet = EncodingPacket::deserialize(&chunk.data);
                        if let Some(result_data) = dec.decode(packet) {
                            // Success!
                            let mut final_data = result_data;
                            if let Some(len) = self.raptorq_transfer_length {
                                final_data.truncate(len as usize);
                            }

                            match self.finalize_raptorq(final_data) {
                                Ok((filename, data)) => {
                                    return self.make_result(ScanStatus::Complete, filename, data)
                                }
                                Err(_) => {
                                    return self.make_result(
                                        ScanStatus::Error,
                                        "Decompress failed".to_string(),
                                        vec![],
                                    )
                                }
                            }
                        }
                    }
                    return self.current_status(ScanStatus::ChunkFound);
                }
            }
            DecodeMode::Unknown => unreachable!(),
        }

        self.current_status(ScanStatus::Scanning)
    }

    fn finalize_raptorq(&self, data: Vec<u8>) -> anyhow::Result<(String, Vec<u8>)> {
        let packed = decompress(&data)?;
        unpack_data(&packed)
    }

    fn current_status(&self, status: ScanStatus) -> ScanResult {
        self.make_result(status, String::new(), vec![])
    }

    fn make_result(&self, status: ScanStatus, filename: String, file_data: Vec<u8>) -> ScanResult {
        let total = self.total_chunks.unwrap_or(0);
        let current = self.chunks.len() as u32;
        ScanResult {
            status,
            progress_current: current,
            progress_total: total,
            filename,
            file_data,
        }
    }
}