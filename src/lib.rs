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

    #[test]
    fn field_filter_keeps_only_matching_lines() {
        let input =
            b"{\"level\":\"info\",\"msg\":\"starting\"}\n{\"level\":\"error\",\"msg\":\"boom\"}\n"
                as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("level=error").unwrap()];

        run(input, &mut output, &filters).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"level\":\"error\",\"msg\":\"boom\"}\n"
        );
    }

    #[test]
    fn field_filter_matches_dotted_nested_path() {
        let input =
            b"{\"meta\":{\"level\":\"error\"}}\n{\"meta\":{\"level\":\"info\"}}\n" as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("meta.level=error").unwrap()];

        run(input, &mut output, &filters).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"meta\":{\"level\":\"error\"}}\n"
        );
    }

    #[test]
    fn field_filter_matches_non_string_values() {
        let input = b"{\"code\":404}\n{\"code\":200}\n" as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("code=404").unwrap()];

        run(input, &mut output, &filters).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "{\"code\":404}\n");
    }

    #[test]
    fn field_filter_drops_lines_missing_the_path() {
        let input = b"{\"level\":\"error\"}\n" as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("meta.level=error").unwrap()];

        run(input, &mut output, &filters).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "");
    }

    #[test]
    fn multiple_field_filters_are_combined_with_and() {
        let input = b"{\"level\":\"error\",\"svc\":\"api\"}\n{\"level\":\"error\",\"svc\":\"db\"}\n"
            as &[u8];
        let mut output = Vec::new();
        let filters = [
            FieldFilter::parse("level=error").unwrap(),
            FieldFilter::parse("svc=api").unwrap(),
        ];

        run(input, &mut output, &filters).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"level\":\"error\",\"svc\":\"api\"}\n"
        );
    }

    #[test]
    fn field_filter_parse_rejects_missing_equals() {
        assert!(FieldFilter::parse("level").is_err());
    }

    #[test]
    fn field_filter_parse_rejects_empty_path() {
        assert!(FieldFilter::parse("=error").is_err());
    }
}
