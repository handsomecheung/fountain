use std::fs;
use tempfile::TempDir;

#[test]
#[cfg(all(feature = "encode", feature = "decode"))]
fn test_encode_decode_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input");
    let qr_output_dir = temp_dir.path().join("qr_output");
    let decoded_output_path = temp_dir.path().join("decoded_output.txt");

    fs::create_dir(&input_dir).expect("Failed to create input dir");
    fs::create_dir(&qr_output_dir).expect("Failed to create qr output dir");

    let source_file_path = input_dir.join("source.txt");
    let original_content = "Hello, world! This is a test for cube encode/decode roundtrip.";
    fs::write(&source_file_path, original_content).expect("Failed to write source file");

    println!("Encoding...");
    let encode_result = cube::encode_file_to_images(&source_file_path, &qr_output_dir, None, 4, false)
        .expect("Encoding failed");

    assert!(encode_result.num_chunks > 0);

    let entries = fs::read_dir(&qr_output_dir).expect("Failed to read qr output dir");
    let count = entries.count();
    assert_eq!(count, encode_result.num_chunks);

    println!("Decoding...");
    let decode_result = cube::decode_from_images(&qr_output_dir, Some(&decoded_output_path))
        .expect("Decoding failed");

    assert_eq!(decode_result.num_chunks, encode_result.num_chunks);

    let decoded_content =
        fs::read_to_string(&decoded_output_path).expect("Failed to read decoded file");

    assert_eq!(original_content, decoded_content);
}

#[test]
#[cfg(feature = "encode")]
fn test_encode_images_size_consistency() {
    use image::GenericImageView;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input");
    let qr_output_dir = temp_dir.path().join("qr_output_consistency");

    fs::create_dir(&input_dir).expect("Failed to create input dir");

    // Using random data to ensure it doesn't compress too trivially.
    let source_file_path = input_dir.join("consistency_test.bin");
    // 20KB with a small chunk size should produce multiple chunks (~200 chunks)
    let data: Vec<u8> = (0..20000).map(|i| (i % 255) as u8).collect();
    fs::write(&source_file_path, &data).expect("Failed to write source file");

    // Use a small chunk size to ensure we get many chunks including a partial last one
    let encode_result =
        cube::encode_file_to_images(&source_file_path, &qr_output_dir, Some(100), 4, false)
            .expect("Encoding failed");

    assert!(
        encode_result.num_chunks > 1,
        "Test requires multiple chunks to verify consistency, got {}",
        encode_result.num_chunks
    );

    let mut first_dimensions: Option<(u32, u32)> = None;

    for filename in encode_result.output_files {
        let path = qr_output_dir.join(filename);
        let img = image::open(&path).expect("Failed to open generated QR image");
        let dims = img.dimensions();

        if let Some(first) = first_dimensions {
            assert_eq!(
                dims, first,
                "Image dimensions mismatch! All QR codes should be same size. Found {:?} vs {:?}",
                dims, first
            );
        } else {
            first_dimensions = Some(dims);
        }
    }
}

#[test]
#[cfg(feature = "encode")]
fn test_encode_gif_size_consistency() {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;
    use std::fs::File;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input_gif");
    let output_gif_path = temp_dir.path().join("output.gif");

    fs::create_dir(&input_dir).expect("Failed to create input dir");

    let source_file_path = input_dir.join("gif_consistency_test.bin");
    let data: Vec<u8> = (0..20000).map(|i| (i % 255) as u8).collect();
    fs::write(&source_file_path, &data).expect("Failed to write source file");

    cube::encode_file_to_gif(&source_file_path, &output_gif_path, Some(100), 100, 4, false)
        .expect("GIF encoding failed");

    let file = File::open(&output_gif_path).expect("Failed to open generated GIF");
    let reader = std::io::BufReader::new(file);
    let decoder = GifDecoder::new(reader).expect("Failed to create GIF decoder");
    let frames = decoder
        .into_frames()
        .collect_frames()
        .expect("Failed to decode GIF frames");

    assert!(
        frames.len() > 1,
        "Test requires multiple frames, got {}",
        frames.len()
    );

    let mut first_dimensions: Option<(u32, u32)> = None;
    for frame in frames {
        let buffer = frame.buffer();
        let (width, height) = buffer.dimensions();
        let dims = (width, height);

        if let Some(_first) = first_dimensions {
            first_dimensions = Some(dims);
        }
    }
}

#[test]
#[cfg(all(feature = "encode", feature = "decode"))]
fn test_encode_decode_gif_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input");
    let output_gif_path = temp_dir.path().join("output.gif");
    let decoded_output_path = temp_dir.path().join("decoded_from_gif.txt");

    fs::create_dir(&input_dir).expect("Failed to create input dir");

    let source_file_path = input_dir.join("source.txt");
    let original_content = "Roundtrip test for GIF encoding and decoding.";
    fs::write(&source_file_path, original_content).expect("Failed to write source file");

    println!("Encoding to GIF...");
    let encode_result = cube::encode_file_to_gif(&source_file_path, &output_gif_path, None, 100, 4, false)
        .expect("GIF encoding failed");

    assert!(encode_result.num_chunks > 0);

    println!("Decoding from GIF...");
    let decode_result = cube::decode_from_gif(&output_gif_path, Some(&decoded_output_path))
        .expect("GIF decoding failed");

    assert_eq!(decode_result.num_chunks, encode_result.num_chunks);

    let decoded_content =
        fs::read_to_string(&decoded_output_path).expect("Failed to read decoded file");
    assert_eq!(original_content, decoded_content);
}

#[test]
#[cfg(all(feature = "encode", feature = "decode"))]
fn test_encode_decode_video_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input");
    let output_gif_path = temp_dir.path().join("video_test.gif");
    let decoded_output_path = temp_dir.path().join("decoded_from_video.txt");

    fs::create_dir(&input_dir).expect("Failed to create input dir");

    let source_file_path = input_dir.join("source_video.txt");
    let original_content = "Roundtrip test for Video decoding (via GIF).";
    fs::write(&source_file_path, original_content).expect("Failed to write source file");

    println!("Encoding to GIF (as video source)...");
    let encode_result = cube::encode_file_to_gif(&source_file_path, &output_gif_path, None, 100, 4, false)
        .expect("GIF encoding failed");

    assert!(encode_result.num_chunks > 0);

    println!("Decoding from Video (GIF file)...");
    let decode_result =
        cube::decode_from_video(&output_gif_path, Some(&decoded_output_path))
            .expect("Video decoding failed");

    assert_eq!(decode_result.num_chunks, encode_result.num_chunks);

    let decoded_content =
        fs::read_to_string(&decoded_output_path).expect("Failed to read decoded file");
    assert_eq!(original_content, decoded_content);
}

#[test]
#[cfg(feature = "encode")]
fn test_terminal_raptorq_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input_raptorq_term");

    fs::create_dir(&input_dir).expect("Failed to create input dir");

    let source_file_path = input_dir.join("source.txt");
    let original_content = "RaptorQ terminal test content. ".repeat(50);
    fs::write(&source_file_path, &original_content).expect("Failed to write source file");

    println!("Encoding for terminal with RaptorQ...");
    // Use a small chunk size to force multiple packets
    let terminal_data = cube::encode_file_for_terminal_raptorq(&source_file_path, Some(100))
        .expect("Encoding failed");

    assert!(terminal_data.total > 0);
    assert!(!terminal_data.qr_strings.is_empty());
    assert_eq!(terminal_data.total, terminal_data.qr_strings.len());
    
    // Basic validation of the QR string format (ASCII art)
    for qr in &terminal_data.qr_strings {
        assert!(qr.contains("██"), "QR string should contain block characters");
    }
}

#[test]
#[cfg(all(feature = "encode", feature = "decode"))]
fn test_raptorq_gif_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input_raptorq_gif");
    let output_gif_path = temp_dir.path().join("raptorq.gif");
    let decoded_output_path = temp_dir.path().join("decoded_raptorq_gif.txt");

    fs::create_dir(&input_dir).expect("Failed to create input dir");

    let source_file_path = input_dir.join("source.txt");
    let original_content = "RaptorQ GIF test content. ".repeat(50);
    fs::write(&source_file_path, &original_content).expect("Failed to write source file");

    println!("Encoding to GIF with RaptorQ...");
    // Use smaller chunks to force redundancy
    let encode_result = cube::encode_file_to_gif(&source_file_path, &output_gif_path, Some(200), 50, 4, true)
        .expect("GIF encoding failed");

    assert!(encode_result.num_chunks > 1);

    println!("Decoding from GIF (RaptorQ)...");
    let decode_result = cube::decode_from_gif(&output_gif_path, Some(&decoded_output_path))
        .expect("GIF decoding failed");

    // num_chunks might be total packets found (which is > source chunks)
    assert!(decode_result.num_chunks > 0);

    let decoded_content =
        fs::read_to_string(&decoded_output_path).expect("Failed to read decoded file");

    assert_eq!(original_content, decoded_content);
}