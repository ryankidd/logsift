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

    /// Only pass through lines where PATH equals VALUE, e.g. `level=error`
    /// or `meta.level=error` for a nested field. Repeatable: a line must
    /// satisfy every given filter.
    #[arg(long = "field", value_name = "PATH=VALUE")]
    field: Vec<String>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let filters = cli
        .field
        .iter()
        .map(|spec| logsift::FieldFilter::parse(spec))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| {
            eprintln!("logsift: {err}");
            std::process::exit(2);
        });

    let stdout = io::stdout();
    let mut out = stdout.lock();

    match cli.path {
        Some(path) => {
            let reader = BufReader::new(File::open(path)?);
            logsift::run(reader, &mut out, &filters)
        }
        None => {
            let stdin = io::stdin();
            logsift::run(stdin.lock(), &mut out, &filters)
        }
    }
}
