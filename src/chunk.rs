use anyhow::{anyhow, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

// Default chunk size for QR code generation
// Smaller = smaller QR codes but more of them
// Larger = larger QR codes but fewer of them
//
// QR code size reference (binary mode, M error correction):
//   ~100 bytes -> ~29x29 modules (fits in small terminal)
//   ~200 bytes -> ~37x37 modules
//   ~500 bytes -> ~53x53 modules
//   ~1400 bytes -> ~73x73 modules (original default)
pub const DEFAULT_PAYLOAD_SIZE: usize = 100; // Small default for terminal display
pub const MAX_PAYLOAD_SIZE: usize = 1400; // Max for file output
pub const CHECKSUM_SIZE: usize = 8;
pub const V0_HEADER_SIZE: usize = 9; // 1 (version) + 4 (total) + 4 (index)
pub const V1_HEADER_SIZE: usize = 11; // 1 (version) + 4 (transfer len) + 4 (esi) + 2 (packet size)

#[derive(Debug, Clone)]
pub struct ChunkHeader {
    pub version: u8,
    pub total: u32,       // V0: Total Chunks, V1: Transfer Length
    pub index: u32,       // V0: Index, V1: ESI
    pub packet_size: u16, // V0: Unused, V1: Packet Size
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub header: ChunkHeader,
    pub data: Vec<u8>,
}

impl ChunkHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self.version {
            0 => {
                let mut bytes = vec![0u8; V0_HEADER_SIZE];
                bytes[0] = self.version;
                bytes[1..5].copy_from_slice(&self.total.to_be_bytes());
                bytes[5..9].copy_from_slice(&self.index.to_be_bytes());
                bytes
            }
            1 => {
                let mut bytes = vec![0u8; V1_HEADER_SIZE];
                bytes[0] = self.version;
                bytes[1..5].copy_from_slice(&self.total.to_be_bytes());
                bytes[5..9].copy_from_slice(&self.index.to_be_bytes());
                bytes[9..11].copy_from_slice(&self.packet_size.to_be_bytes());
                bytes
            }
            _ => panic!("Unsupported version for encoding: {}", self.version),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize)> {
        if bytes.is_empty() {
            return Err(anyhow!("Invalid header: empty"));
        }
        let version = bytes[0];
        match version {
            0 => {
                if bytes.len() < V0_HEADER_SIZE {
                    return Err(anyhow!("Invalid V0 header: too short"));
                }
                let total = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
                let index = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
                Ok((
                    ChunkHeader {
                        version,
                        total,
                        index,
                        packet_size: 0,
                    },
                    V0_HEADER_SIZE,
                ))
            }
            1 => {
                if bytes.len() < V1_HEADER_SIZE {
                    return Err(anyhow!("Invalid V1 header: too short"));
                }
                let total = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
                let index = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
                let packet_size = u16::from_be_bytes([bytes[9], bytes[10]]);
                Ok((
                    ChunkHeader {
                        version,
                        total,
                        index,
                        packet_size,
                    },
                    V1_HEADER_SIZE,
                ))
            }
            _ => Err(anyhow!("Unsupported chunk version: {}", version)),
        }
    }
}

impl Chunk {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let header_bytes = self.header.to_bytes();
        let mut result = Vec::with_capacity(header_bytes.len() + self.data.len());
        result.extend_from_slice(&header_bytes);
        result.extend_from_slice(&self.data);
        Ok(result)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (header, header_len) = ChunkHeader::from_bytes(bytes)?;
        let data = bytes[header_len..].to_vec();

        Ok(Chunk { header, data })
    }
}

pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

pub fn calculate_checksum(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result[..CHECKSUM_SIZE].to_vec()
}

// Pack data: [Checksum 8B] [Filename] [\0] [Content]
pub fn pack_data(data: &[u8], filename: &str) -> Vec<u8> {
    let checksum = calculate_checksum(data);
    // Sanitize filename: remove null bytes
    let clean_filename = filename.replace('\0', "");

    let mut packed = Vec::with_capacity(CHECKSUM_SIZE + clean_filename.len() + 1 + data.len());
    packed.extend_from_slice(&checksum);
    packed.extend_from_slice(clean_filename.as_bytes());
    packed.push(0); // Null terminator
    packed.extend_from_slice(data);
    packed
}

// Unpack data: -> (Filename, Content)
pub fn unpack_data(packed: &[u8]) -> Result<(String, Vec<u8>)> {
    if packed.len() < CHECKSUM_SIZE + 2 {
        // Min: Checksum + 1 char + \0
        return Err(anyhow!("Invalid packed data: too short"));
    }

    let expected_checksum = &packed[..CHECKSUM_SIZE];

    let mut null_pos = None;
    for i in CHECKSUM_SIZE..packed.len() {
        if packed[i] == 0 {
            null_pos = Some(i);
            break;
        }
    }

    let null_idx =
        null_pos.ok_or_else(|| anyhow!("Invalid packed data: missing filename terminator"))?;

    let filename_bytes = &packed[CHECKSUM_SIZE..null_idx];
    let filename = std::str::from_utf8(filename_bytes)
        .map_err(|_| anyhow!("Invalid filename: not valid UTF-8"))?
        .to_string();

    let content = packed[null_idx + 1..].to_vec();

    let actual_checksum = calculate_checksum(&content);
    if actual_checksum != expected_checksum {
        return Err(anyhow!(
            "Checksum mismatch: expected {:?}, got {:?}",
            expected_checksum,
            actual_checksum
        ));
    }

    Ok((filename, content))
}

pub fn split_into_chunks(data: &[u8], filename: &str) -> Result<Vec<Chunk>> {
    split_into_chunks_with_size(data, filename, MAX_PAYLOAD_SIZE)
}

pub fn split_into_chunks_with_size(
    data: &[u8],
    filename: &str,
    payload_size: usize,
) -> Result<Vec<Chunk>> {
    let packed = pack_data(data, filename);
    let compressed = compress(&packed)?;
    Ok(split_compressed_into_chunks(&compressed, payload_size).collect())
}

pub struct ChunkIterator<'a> {
    compressed: &'a [u8],
    payload_size: usize,
    total_chunks: u32,
    current_index: usize,
    is_empty_input: bool,
    finished: bool,
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        if self.is_empty_input {
            self.finished = true;
            return Some(Chunk {
                header: ChunkHeader {
                    version: 0,
                    total: 1,
                    index: 0,
                    packet_size: 0,
                },
                data: Vec::new(),
            });
        }

        if self.current_index as u32 >= self.total_chunks {
            self.finished = true;
            return None;
        }

        let start = self.current_index * self.payload_size;
        let end = (start + self.payload_size).min(self.compressed.len());
        let chunk_data = &self.compressed[start..end];

        let chunk = Chunk {
            header: ChunkHeader {
                version: 0,
                total: self.total_chunks,
                index: self.current_index as u32,
                packet_size: 0,
            },
            data: chunk_data.to_vec(),
        };

        self.current_index += 1;
        Some(chunk)
    }
}

pub fn split_compressed_into_chunks(compressed: &[u8], payload_size: usize) -> ChunkIterator<'_> {
    let total_chunks = (compressed.len() + payload_size - 1) / payload_size;
    let total_chunks = total_chunks.max(1) as u32;

    ChunkIterator {
        compressed,
        payload_size,
        total_chunks,
        current_index: 0,
        is_empty_input: compressed.is_empty(),
        finished: false,
    }
}

pub fn merge_chunks(mut chunks: Vec<Chunk>) -> Result<(String, Vec<u8>)> {
    if chunks.is_empty() {
        return Err(anyhow!("No chunks to merge"));
    }

    chunks.sort_by_key(|c| c.header.index);

    let expected_total = chunks[0].header.total;

    if chunks.len() as u32 != expected_total {
        return Err(anyhow!(
            "Missing chunks: expected {}, got {}",
            expected_total,
            chunks.len()
        ));
    }

    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.header.index != i as u32 {
            return Err(anyhow!("Missing chunk at index {}", i));
        }
    }

    let mut compressed_data = Vec::new();
    for chunk in chunks {
        compressed_data.extend_from_slice(&chunk.data);
    }

    let packed = decompress(&compressed_data)?;
    unpack_data(&packed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_roundtrip() {
        let data = b"Hello, World! This is a test.";
        let chunks = split_into_chunks(data, "test.txt").unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].header.total, 1);
        assert_eq!(chunks[0].header.index, 0);

        let (filename, restored) = merge_chunks(chunks).unwrap();
        assert_eq!(filename, "test.txt");
        assert_eq!(restored, data);
    }

    #[test]
    fn test_large_data_chunking() {
        // Use data large enough to require multiple chunks even after compression
        let mut x: u64 = 12345;
        let data: Vec<u8> = (0..100000)
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                (x >> 56) as u8
            })
            .collect();
        let chunks = split_into_chunks(&data, "large.bin").unwrap();

        assert!(
            chunks.len() > 1,
            "Expected multiple chunks, got {}",
            chunks.len()
        );

        let (filename, restored) = merge_chunks(chunks).unwrap();
        assert_eq!(filename, "large.bin");
        assert_eq!(restored, data);
    }

    #[test]
    fn test_pack_unpack() {
        let data = b"Some random data";
        let filename = "example.file";

        let packed = pack_data(data, filename);
        let (name, content) = unpack_data(&packed).unwrap();

        assert_eq!(name, filename);
        assert_eq!(content, data);
    }
}
