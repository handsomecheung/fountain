pub mod chunk;

#[cfg(feature = "decode")]
pub mod decode;

#[cfg(feature = "encode")]
pub mod encode;

pub mod qr;

#[cfg(feature = "encode")]
pub mod terminal;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use chunk::{
    split_into_chunks_with_size, Chunk, ChunkHeader, DEFAULT_PAYLOAD_SIZE, MAX_PAYLOAD_SIZE,
};

#[cfg(feature = "decode")]
pub use decode::{decode_from_gif, decode_from_images, decode_from_video, DecodeResult};

#[cfg(feature = "encode")]
pub use encode::{
    encode_file_for_terminal, encode_file_to_gif, encode_file_to_images, EncodeResult,
    TerminalQrData,
};

#[cfg(feature = "encode")]
pub use terminal::{display_qr_carousel, display_qr_once};
