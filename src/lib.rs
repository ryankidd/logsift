use std::io::{self, BufRead, Write};

/// Reads newline-delimited JSON from `input`, validates each non-blank
/// line as JSON, and writes valid lines back out to `output` unchanged.
/// Invalid lines are reported on stderr and dropped.
pub fn run<R: BufRead, W: Write>(input: R, output: &mut W) -> io::Result<()> {
    for (line_number, line) in input.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(_) => writeln!(output, "{line}")?,
            Err(err) => {
                eprintln!("logsift: line {}: invalid JSON: {err}", line_number + 1);
            }
        }
    }

    Ok(())
}
