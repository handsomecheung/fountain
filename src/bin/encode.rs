use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use cube::{
    display_qr_carousel, display_qr_once, encode_file, encode_file_for_terminal,
    DEFAULT_PAYLOAD_SIZE, MAX_PAYLOAD_SIZE,
};

#[derive(Parser)]
#[command(name = "cube-encode")]
#[command(author, version, about = "Encode files to QR codes", long_about = None)]
struct Cli {
    /// Input file to encode
    input: PathBuf,

    /// Output directory for QR code images
    #[arg(short = 'm', long = "image-output-dir", required_unless_present = "terminal")]
    image_output_dir: Option<PathBuf>,

    /// Display QR codes in terminal instead of saving to files
    #[arg(short, long)]
    terminal: bool,

    /// Interval in milliseconds for auto-switching QR codes in terminal mode (default: 2000)
    #[arg(short, long, default_value = "2000")]
    interval: u64,

    /// Show all QR codes at once without carousel (only with --terminal)
    #[arg(long)]
    no_carousel: bool,

    /// Maximum payload size (bytes) per QR code. Smaller values make QR codes less dense and easier to scan.
    /// Default is ~1400 for file output (high density) and 100 for terminal.
    #[arg(short = 's', long, alias = "payload-size")]
    chunk_size: Option<usize>,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    if args.terminal {
        println!(
            "Encoding file for terminal display: {}",
            args.input.display()
        );

        let data = encode_file_for_terminal(&args.input, args.chunk_size)?;

        println!("Generated {} QR code(s)", data.total);

        let requested_size = args.chunk_size.unwrap_or(DEFAULT_PAYLOAD_SIZE);
        if data.effective_size < requested_size {
            println!(
                "⚠️  Automatically reduced payload size to {} bytes to fit terminal.",
                data.effective_size
            );
        }
        println!();

        if args.no_carousel || data.total == 1 {
            display_qr_once(&data);
        } else {
            println!("Starting carousel mode ({}ms interval)...", args.interval);
            println!("Press Ctrl+C to exit");
            std::thread::sleep(std::time::Duration::from_secs(1));
            display_qr_carousel(&data, args.interval);
        }
    } else {
        let output_dir = args.image_output_dir.expect("Required by clap logic");
        println!("Encoding file: {}", args.input.display());
        println!("Output directory: {}", output_dir.display());
        if let Some(size) = args.chunk_size {
            println!("Max payload size: {} bytes", size);
        }

        let result = encode_file(&args.input, &output_dir, args.chunk_size)?;

        let requested_size = args.chunk_size.unwrap_or(MAX_PAYLOAD_SIZE);
        if result.effective_size < requested_size {
            println!();
            println!(
                "⚠️  Automatically reduced payload size to {} bytes to fit QR code capacity.",
                result.effective_size
            );
        }

        println!();
        println!("Successfully created {} QR code(s)", result.num_chunks);
    }

    Ok(())
}
