use logsync::application::compare_logfiles;
use logsync::domain::LogFile;
use logsync::tui::start;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <first.log> <second.log> [more.log ...]");
        std::process::exit(2);
    }

    let logfiles: Vec<_> = args.into_iter().map(LogFile::from_file).collect();
    let result = compare_logfiles(logfiles.iter().collect());
    start(&result)
}
