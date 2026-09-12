mod application;
mod domain;
mod tui;

use crate::application::compare_logfiles::compare_logfiles;
use crate::domain::LogFile;
use crate::tui::start;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <left.log> <right.log>");
        std::process::exit(2);
    }

    let left = LogFile::from_file(args[0].clone());
    let right = LogFile::from_file(args[1].clone());
    let result = compare_logfiles(&left, &right);
    start(&result)
}
