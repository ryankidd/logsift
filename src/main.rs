use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;

use chrono::{DateTime, FixedOffset};
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

    /// Only pass through lines with a timestamp at or after this RFC 3339
    /// instant, e.g. `2024-01-01T00:00:00Z`.
    #[arg(long, value_name = "RFC3339", value_parser = parse_rfc3339)]
    since: Option<DateTime<FixedOffset>>,

    /// Only pass through lines with a timestamp at or before this RFC 3339
    /// instant.
    #[arg(long, value_name = "RFC3339", value_parser = parse_rfc3339)]
    until: Option<DateTime<FixedOffset>>,

    /// Dotted path to the timestamp field checked by `--since`/`--until`.
    #[arg(long = "time-field", value_name = "PATH", default_value = "timestamp")]
    time_field: String,
}

fn parse_rfc3339(s: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(s).map_err(|err| format!("invalid RFC3339 timestamp {s:?}: {err}"))
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

    let time_filter = (cli.since.is_some() || cli.until.is_some())
        .then(|| logsift::TimeFilter::new(&cli.time_field, cli.since, cli.until));

    let stdout = io::stdout();
    let mut out = stdout.lock();

    match cli.path {
        Some(path) => {
            let reader = BufReader::new(File::open(path)?);
            logsift::run(reader, &mut out, &filters, time_filter.as_ref())
        }
        None => {
            let stdin = io::stdin();
            logsift::run(stdin.lock(), &mut out, &filters, time_filter.as_ref())
        }
    }
}
