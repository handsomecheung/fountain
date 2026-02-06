use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use fountain::{decode_from_gif, decode_from_images, decode_from_video};

#[derive(Parser)]
#[command(name = "fountain-decode")]
#[command(author, version, about = "Decode QR code images or video back to original file", long_about = None)]
struct Cli {
    /// Input directory or video file
    input: PathBuf,

    /// Output file path (defaults to original filename in current directory)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    if !args.input.exists() {
        anyhow::bail!("Input path does not exist: {}", args.input.display());
    }

    let result = if args.input.is_dir() {
        println!("Decoding QR codes from directory: {}", args.input.display());
        decode_from_images(&args.input, args.output.as_deref())?
    } else {
        let is_gif = args
            .input
            .extension()
            .map(|ext| ext.to_ascii_lowercase() == "gif")
            .unwrap_or(false);

        if is_gif {
            decode_from_gif(&args.input, args.output.as_deref())?
        } else {
            println!(
                "Decoding QR codes from video file: {}",
                args.input.display()
            );
            decode_from_video(&args.input, args.output.as_deref())?
        }
    };

    println!();
    println!("Successfully decoded {} QR code(s)", result.num_chunks);
    println!("Original filename: {}", result.original_filename);
    println!("Output file: {}", result.output_path);

    Ok(())
}
