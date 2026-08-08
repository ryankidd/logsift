use std::io::{self, BufRead, Write};

use serde_json::Value;

/// A `--field PATH=VALUE` filter matched against a dotted path into a JSON
/// object, e.g. `level=error` or `meta.level=error` for nested fields.
#[derive(Debug, PartialEq, Eq)]
pub struct FieldFilter {
    path: Vec<String>,
    value: String,
}

impl FieldFilter {
    /// Parses a filter spec of the form `PATH=VALUE`.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let (path, value) = spec
            .split_once('=')
            .ok_or_else(|| format!("invalid --field {spec:?}: expected PATH=VALUE"))?;
        if path.is_empty() {
            return Err(format!("invalid --field {spec:?}: PATH is empty"));
        }

        Ok(FieldFilter {
            path: path.split('.').map(str::to_string).collect(),
            value: value.to_string(),
        })
    }

    fn matches(&self, root: &Value) -> bool {
        let mut current = root;
        for key in &self.path {
            match current.get(key) {
                Some(next) => current = next,
                None => return false,
            }
        }

        match current {
            Value::String(s) => *s == self.value,
            Value::Null => false,
            // Compare the JSON-encoded form so numbers/bools can be
            // matched by their literal text, e.g. `--field code=404`.
            #[allow(clippy::cmp_owned)]
            other => other.to_string() == self.value,
        }
    }
}

/// Reads newline-delimited JSON from `input`, validates each non-blank
/// line as JSON, and writes lines that satisfy every filter in `filters`
/// back out to `output` unchanged. Invalid lines are reported on stderr
/// and dropped.
pub fn run<R: BufRead, W: Write>(
    input: R,
    output: &mut W,
    filters: &[FieldFilter],
) -> io::Result<()> {
    for (line_number, line) in input.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                if filters.iter().all(|f| f.matches(&value)) {
                    writeln!(output, "{line}")?;
                }
            }
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

        run(input, &mut output, &[]).unwrap();

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

        run(input, &mut output, &[]).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "{\"ok\":true}\n");
    }
}
