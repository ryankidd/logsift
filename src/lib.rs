use std::io::{self, BufRead, Write};

use chrono::{DateTime, FixedOffset};
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

/// A `--since`/`--until` time-range filter matched against a dotted path
/// into a JSON object, e.g. the default `timestamp` field or a nested one
/// like `meta.timestamp`. Bounds are inclusive; a line whose timestamp
/// field is missing or not a valid RFC 3339 string is excluded.
#[derive(Debug)]
pub struct TimeFilter {
    field: Vec<String>,
    since: Option<DateTime<FixedOffset>>,
    until: Option<DateTime<FixedOffset>>,
}

impl TimeFilter {
    /// Dotted path used for the timestamp field when `--time-field` is not
    /// given.
    pub const DEFAULT_FIELD: &'static str = "timestamp";

    /// Builds a filter that reads the timestamp from `field` (a dotted
    /// path) and keeps lines whose timestamp falls within `[since, until]`.
    /// Either bound may be omitted.
    pub fn new(
        field: &str,
        since: Option<DateTime<FixedOffset>>,
        until: Option<DateTime<FixedOffset>>,
    ) -> Self {
        TimeFilter {
            field: field.split('.').map(str::to_string).collect(),
            since,
            until,
        }
    }

    fn matches(&self, root: &Value) -> bool {
        let mut current = root;
        for key in &self.field {
            match current.get(key) {
                Some(next) => current = next,
                None => return false,
            }
        }

        let Some(text) = current.as_str() else {
            return false;
        };
        let Ok(timestamp) = DateTime::parse_from_rfc3339(text) else {
            return false;
        };

        if self.since.is_some_and(|since| timestamp < since) {
            return false;
        }
        if self.until.is_some_and(|until| timestamp > until) {
            return false;
        }

        true
    }
}

/// Reads newline-delimited JSON from `input`, validates each non-blank
/// line as JSON, and writes lines that satisfy every filter in `filters`
/// and `time_filter` (if given) back out to `output` unchanged. Invalid
/// lines are reported on stderr and dropped.
pub fn run<R: BufRead, W: Write>(
    input: R,
    output: &mut W,
    filters: &[FieldFilter],
    time_filter: Option<&TimeFilter>,
) -> io::Result<()> {
    for (line_number, line) in input.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                let time_ok = match time_filter {
                    Some(time_filter) => time_filter.matches(&value),
                    None => true,
                };

                if time_ok && filters.iter().all(|f| f.matches(&value)) {
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

        run(input, &mut output, &[], None).unwrap();

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

        run(input, &mut output, &[], None).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "{\"ok\":true}\n");
    }

    #[test]
    fn field_filter_keeps_only_matching_lines() {
        let input =
            b"{\"level\":\"info\",\"msg\":\"starting\"}\n{\"level\":\"error\",\"msg\":\"boom\"}\n"
                as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("level=error").unwrap()];

        run(input, &mut output, &filters, None).unwrap();

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

        run(input, &mut output, &filters, None).unwrap();

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

        run(input, &mut output, &filters, None).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "{\"code\":404}\n");
    }

    #[test]
    fn field_filter_drops_lines_missing_the_path() {
        let input = b"{\"level\":\"error\"}\n" as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("meta.level=error").unwrap()];

        run(input, &mut output, &filters, None).unwrap();

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

        run(input, &mut output, &filters, None).unwrap();

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

    fn rfc3339(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    #[test]
    fn time_filter_since_keeps_lines_at_or_after_bound() {
        let input =
            b"{\"timestamp\":\"2024-01-01T00:00:00Z\"}\n{\"timestamp\":\"2024-01-03T00:00:00Z\"}\n"
                as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new("timestamp", Some(rfc3339("2024-01-02T00:00:00Z")), None);

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"timestamp\":\"2024-01-03T00:00:00Z\"}\n"
        );
    }

    #[test]
    fn time_filter_until_keeps_lines_at_or_before_bound() {
        let input =
            b"{\"timestamp\":\"2024-01-01T00:00:00Z\"}\n{\"timestamp\":\"2024-01-03T00:00:00Z\"}\n"
                as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new("timestamp", None, Some(rfc3339("2024-01-02T00:00:00Z")));

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"timestamp\":\"2024-01-01T00:00:00Z\"}\n"
        );
    }

    #[test]
    fn time_filter_since_and_until_keep_lines_within_range() {
        let input = b"{\"timestamp\":\"2024-01-01T00:00:00Z\"}\n{\"timestamp\":\"2024-01-02T00:00:00Z\"}\n{\"timestamp\":\"2024-01-03T00:00:00Z\"}\n"
            as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new(
            "timestamp",
            Some(rfc3339("2024-01-02T00:00:00Z")),
            Some(rfc3339("2024-01-02T00:00:00Z")),
        );

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"timestamp\":\"2024-01-02T00:00:00Z\"}\n"
        );
    }

    #[test]
    fn time_filter_bounds_are_inclusive() {
        let bound = rfc3339("2024-01-02T00:00:00Z");
        let input = b"{\"timestamp\":\"2024-01-02T00:00:00Z\"}\n" as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new("timestamp", Some(bound), Some(bound));

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"timestamp\":\"2024-01-02T00:00:00Z\"}\n"
        );
    }

    #[test]
    fn time_filter_drops_lines_missing_the_field() {
        let input = b"{\"msg\":\"no timestamp here\"}\n" as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new("timestamp", Some(rfc3339("2024-01-01T00:00:00Z")), None);

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "");
    }

    #[test]
    fn time_filter_drops_lines_with_unparseable_timestamp() {
        let input = b"{\"timestamp\":\"not a timestamp\"}\n" as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new("timestamp", Some(rfc3339("2024-01-01T00:00:00Z")), None);

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "");
    }

    #[test]
    fn time_filter_matches_dotted_nested_field() {
        let input = b"{\"meta\":{\"timestamp\":\"2024-01-03T00:00:00Z\"}}\n{\"meta\":{\"timestamp\":\"2024-01-01T00:00:00Z\"}}\n"
            as &[u8];
        let mut output = Vec::new();
        let time_filter = TimeFilter::new(
            "meta.timestamp",
            Some(rfc3339("2024-01-02T00:00:00Z")),
            None,
        );

        run(input, &mut output, &[], Some(&time_filter)).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"meta\":{\"timestamp\":\"2024-01-03T00:00:00Z\"}}\n"
        );
    }

    #[test]
    fn time_filter_and_field_filter_combine_with_and() {
        let input = b"{\"level\":\"error\",\"timestamp\":\"2024-01-01T00:00:00Z\"}\n{\"level\":\"error\",\"timestamp\":\"2024-01-03T00:00:00Z\"}\n{\"level\":\"info\",\"timestamp\":\"2024-01-03T00:00:00Z\"}\n"
            as &[u8];
        let mut output = Vec::new();
        let filters = [FieldFilter::parse("level=error").unwrap()];
        let time_filter = TimeFilter::new("timestamp", Some(rfc3339("2024-01-02T00:00:00Z")), None);

        run(input, &mut output, &filters, Some(&time_filter)).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"level\":\"error\",\"timestamp\":\"2024-01-03T00:00:00Z\"}\n"
        );
    }
}
