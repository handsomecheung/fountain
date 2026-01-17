use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use cube::{display_qr_carousel, display_qr_once, encode_file, encode_file_for_terminal};

#[derive(Parser)]
#[command(name = "cube-encode")]
#[command(author, version, about = "Encode files to QR codes", long_about = None)]
struct Cli {
    /// Input file to encode
    input: PathBuf,

    /// Output directory for QR code images
    #[arg(short, long, default_value = "./qr_output")]
    output: PathBuf,

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
        println!("Encoding file for terminal display: {}", args.input.display());

        let data = encode_file_for_terminal(&args.input)?;

        println!("Generated {} QR code(s)", data.total);
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
        println!("Encoding file: {}", args.input.display());
        println!("Output directory: {}", args.output.display());
        if let Some(size) = args.chunk_size {
            println!("Max payload size: {} bytes", size);
        }

        let result = encode_file(&args.input, &args.output, args.chunk_size)?;

        println!();
        println!("Successfully created {} QR code(s)", result.num_chunks);
        println!("Output files:");
        for file in &result.output_files {
            println!("  - {}", file);
        }
    }

    Ok(())
}
