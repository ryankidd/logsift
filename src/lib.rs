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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_through_valid_json_lines() {
        let input =
            b"{\"level\":\"info\",\"msg\":\"starting\"}\n{\"level\":\"error\",\"msg\":\"boom\"}\n"
                as &[u8];
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        let got = String::from_utf8(output).unwrap();
        assert_eq!(
            got,
            "{\"level\":\"info\",\"msg\":\"starting\"}\n{\"level\":\"error\",\"msg\":\"boom\"}\n"
        );
    }

    #[test]
    fn drops_invalid_json_lines() {
        let input = b"{\"ok\":true}\nnot json\n" as &[u8];
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "{\"ok\":true}\n");
    }
}
