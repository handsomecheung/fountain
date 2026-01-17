use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use cube::{
    decode_qr_codes, decode_qr_video, display_qr_carousel, display_qr_once, encode_file,
    encode_file_for_terminal,
};

#[derive(Parser)]
#[command(name = "cube")]
#[command(author, version, about = "Convert files to QR codes and back", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode a file into QR code images
    Encode {
        /// Input file to encode
        input: PathBuf,

        /// Output directory for QR code images
        #[arg(short, long, default_value = "./qr_output")]
        output: PathBuf,

        /// Display QR codes in terminal instead of saving to files
        #[arg(short, long)]
        terminal: bool,

        /// Interval in seconds for auto-switching QR codes in terminal mode (default: 3)
        #[arg(short, long, default_value = "3")]
        interval: u64,

        /// Show all QR codes at once without carousel (only with --terminal)
        #[arg(long)]
        no_carousel: bool,
    },

    /// Decode QR code images or video back to original file
    Decode {
        /// Input directory or video file
        input: PathBuf,

        /// Output file path (defaults to original filename in current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            input,
            output,
            terminal,
            interval,
            no_carousel,
        } => {
            if terminal {
                println!("Encoding file for terminal display: {}", input.display());

                let data = encode_file_for_terminal(&input)?;

                println!("Generated {} QR code(s)", data.total);
                println!();

                if no_carousel || data.total == 1 {
                    display_qr_once(&data);
                } else {
                    println!("Starting carousel mode ({}s interval)...", interval);
                    println!("Press Ctrl+C to exit");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    display_qr_carousel(&data, interval);
                }
            } else {
                println!("Encoding file: {}", input.display());
                println!("Output directory: {}", output.display());

                let result = encode_file(&input, &output)?;

                println!();
                println!("Successfully created {} QR code(s)", result.num_chunks);
                println!("Output files:");
                for file in &result.output_files {
                    println!("  - {}", file);
                }
            }
        }

        Commands::Decode { input, output } => {
            if !input.exists() {
                anyhow::bail!("Input path does not exist: {}", input.display());
            }

            let result = if input.is_dir() {
                println!("Decoding QR codes from directory: {}", input.display());
                decode_qr_codes(&input, output.as_deref())?
            } else {
                println!("Decoding QR codes from video file: {}", input.display());
                decode_qr_video(&input, output.as_deref())?
            };

            println!();
            println!("Successfully decoded {} QR code(s)", result.num_chunks);
            println!("Original filename: {}", result.original_filename);
            println!("Output file: {}", result.output_path);
        }
    }

    Ok(())
}
