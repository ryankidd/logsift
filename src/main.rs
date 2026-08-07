use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;

use clap::Parser;

/// Filter newline-delimited JSON logs.
#[derive(Parser)]
#[command(name = "logsift", version, about)]
struct Cli {
    /// Path to a newline-delimited JSON file. Reads stdin if omitted.
    path: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    match cli.path {
        Some(path) => {
            let reader = BufReader::new(File::open(path)?);
            logsift::run(reader, &mut out)
        }
        None => {
            let stdin = io::stdin();
            logsift::run(stdin.lock(), &mut out)
        }
    }
}
