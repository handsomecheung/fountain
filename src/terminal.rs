use crate::encode::TerminalQrData;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const CLEAR_SCREEN: &str = "\x1B[2J\x1B[H";
const HIDE_CURSOR: &str = "\x1B[?25l";
const SHOW_CURSOR: &str = "\x1B[?25h";

pub fn display_qr_carousel(data: &TerminalQrData, interval_ms: u64) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let total = data.qr_strings.len();

    if total == 1 {
        // Single QR code, just display it
        display_single_qr(&data.qr_strings[0], &data.filename, 1, 1);
        println!("\nPress Ctrl+C to exit...");

        while running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
    } else {
        // Multiple QR codes, carousel mode
        print!("{}", HIDE_CURSOR);
        io::stdout().flush().unwrap();

        let mut current = 0;

        while running.load(Ordering::SeqCst) {
            display_single_qr(&data.qr_strings[current], &data.filename, current + 1, total);
            println!("\nAuto-switching in {}ms | Press Ctrl+C to exit...", interval_ms);

            // Wait for interval or until interrupted
            let start = std::time::Instant::now();
            let duration = Duration::from_millis(interval_ms);
            
            while start.elapsed() < duration {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(std::cmp::min(50, interval_ms)));
            }

            current = (current + 1) % total;
        }

        print!("{}", SHOW_CURSOR);
        io::stdout().flush().unwrap();
    }

    // Clean exit
    print!("{}", CLEAR_SCREEN);
    println!("Exited.");
}

fn display_single_qr(qr_string: &str, filename: &str, current: usize, total: usize) {
    print!("{}", CLEAR_SCREEN);

    println!("File: {}  |  QR Code {}/{}", filename, current, total);
    println!("{}", "=".repeat(50));
    println!();
    println!("{}", qr_string);
}

pub fn display_qr_once(data: &TerminalQrData) {
    let total = data.qr_strings.len();

    for (i, qr_string) in data.qr_strings.iter().enumerate() {
        println!("File: {}  |  QR Code {}/{}", data.filename, i + 1, total);
        println!("{}", "=".repeat(50));
        println!();
        println!("{}", qr_string);

        if i < total - 1 {
            println!();
            println!("{}", "-".repeat(50));
            println!();
        }
    }
}
