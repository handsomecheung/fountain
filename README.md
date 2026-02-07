# Fountain ⛲

Fountain is a high-resilience, air-gapped data transmission tool that converts any file into a stream of QR codes. By leveraging **Fountain Codes (specifically RaptorQ)**, it ensures reliable file transfer via screens, cameras, or paper, even when frames are lost or captured out of order.

## 🌟 The Magic of Fountain Codes in QR Transmission

Traditional file-to-QR methods usually split data into a fixed sequence of frames (e.g., Frame 1 of 10, Frame 2 of 10). If a single frame is missed due to camera flicker, motion blur, or a "dirty" frame, the entire transmission fails or hangs indefinitely waiting for that specific missing piece.

**Fountain Codes change the game:**
- **Order-Independent:** It doesn't matter which QR codes you scan or in what order. 
- **Loss-Tolerant:** If you miss 10% of the frames, you just keep scanning new ones. Any $N + \epsilon$ unique packets are enough to reconstruct the original $N$ blocks of data.
- **Infinite Stream:** The encoder can generate a practically endless stream of unique "fountain" packets. The receiver just "catches" enough drops from the fountain to fill its bucket.

This makes Fountain ideal for **one-way, offline transmission** where the sender cannot hear back from the receiver to retransmit lost packets.

## ✨ Features

- 🚀 **High Resilience:** Uses RaptorQ (RFC 6330) for industrial-grade erasure coding.
- 📱 **Terminal Mode:** Display QR codes directly in your terminal with a carousel effect.
- 🎞️ **GIF & Video Support:** Generate optimized, dither-free GIFs for easy sharing, or decode from recorded video files.
- 🖼️ **Image Export:** Save QR codes as a series of PNG/JPG images.
- 🌐 **Web Scanner (WASM):** Decode QR codes directly in your browser using your phone's camera. Perfect for receiving files on mobile without installing any apps.
- 🛠️ **Configurable:** Adjust pixel scale, payload size, and carousel intervals to match your hardware's capabilities.
- 🦀 **Pure Rust (mostly):** The encoder is lightweight and has zero heavy dependencies.

## 📥 Downloads

Pre-compiled binaries for the **Encoder** and the **Web Scanner (WASM)** are available on the [Releases Page](https://github.com/handsomecheung/fountain/releases/latest). 

- **fountain-encode**: Standalone binaries for Linux/macOS and Windows.
- **fountain-wasm**: Pre-built WASM and JS assets for web deployment.

*Note: The Decoder CLI is currently best used via the portable static build or local cargo build due to OpenCV dependencies.*

## 📦 Installation

### Prerequisites
- **Encoder:** No special requirements.
- **Decoder (CLI):** 
    - **Normal Build:** Requires [OpenCV](https://opencv.org/) development libraries installed on the system (both at compile time and runtime).
    - **Static Build:** If you use the portable static build (via Docker), the resulting binary is standalone and does **not** require OpenCV to be installed on the host system.
- **Decoder (WASM, Web Scanner):** Requires `wasm-bindgen-cli`.

### Build from Source

#### Encoder and Decoder

**Option 1: Portable Static Build (Recommended)**
Builds a standalone binary that includes all dependencies (including OpenCV). This requires Docker.
```bash
./script/build.sh
```

**Option 2: Local Cargo Build**
```bash
# Build both encoder and decoder
cargo build --release

# Build encoder only (no OpenCV needed)
cargo build --release --bin fountain-encode --no-default-features --features encode
```

#### Web Scanner (WASM)

Build the browser-based decoder.
```bash
./script/rust/compile.wasm.sh
```
The output will be in `www/pkg/`.

🌍 Live Demo

Try the Web Scanner directly on your mobile device:
👉 **[fountain.curvekey.app/scanner/](https://fountain.curvekey.app/scanner/)**


## 🚀 Usage

### Encoding (Sender)

```bash
fountain-encode [OPTIONS] <INPUT>
```

**Arguments:**
- `<INPUT>`: Path to the input file you want to encode.

**Options:**
- `-t, --terminal`: Display QR codes directly in your terminal using a carousel.
- `-g, --gif-output-file <FILE>`: Save the QR stream as an optimized animated GIF.
- `-m, --image-output-dir <DIR>`: Export QR codes as a series of individual image files (PNG/JPG).
- `-i, --interval <MS>`: Interval in milliseconds for switching frames in terminal or GIF (default: `2000`).
- `-s, --chunk-size <BYTES>`: Max payload size per QR packet. Smaller values result in simpler, easier-to-scan QR codes but more frames.
- `--pixel-scale <N>`: Scale factor for QR pixels (default: `4`).
- `--no-carousel`: In terminal mode, print all QR codes at once instead of cycling through them.

**Examples:**

*Terminal Carousel (Quickest for one-off transfers):*
```bash
fountain-encode my_secret.key --terminal --interval 500
```

*Generate an optimized GIF:*
```bash
fountain-encode document.pdf -g output.gif --interval 200
```

### Decoding (Receiver)

```bash
fountain-decode [OPTIONS] <INPUT>
```

**Arguments:**
- `<INPUT>`: Path to a video file, a GIF, or a directory containing QR image frames.

**Options:**
- `-o, --output <FILE>`: Path for the reconstructed file. If omitted, uses the original filename.

**Examples:**

*Decode from a video file:*
```bash
fountain-decode recording.mp4 -o restored_file.zip
```

*Decode from a directory of images:*
```bash
fountain-decode ./qr_frames/
```


## 🛠️ How it Works

1. **Chunking:** The file is split into small blocks.
2. **RaptorQ Encoding:** These blocks are transformed into a series of fountain packets. Each packet contains a small piece of the puzzle and metadata describing how it relates to the whole.
3. **Anchor Frame:** For GIFs and videos, Fountain inserts an initial "Anchor Frame" containing the original filename and metadata to help the decoder prepare.
4. **QR Generation:** Each packet is encoded into a high-density QR code.
5. **Reconstruction:** The decoder captures frames, extracts the fountain packets, and once it has enough mathematical overhead (usually < 5% extra), it instantly reconstructs the original file.

## 🧪 Testing

The project includes a suite of integration tests that verify the end-to-end encoding and decoding process.

```bash
./script/test.sh
```

## 📄 License

👉 [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)
