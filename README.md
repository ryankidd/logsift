# logsift

A CLI for filtering structured (newline-delimited JSON) logs.

```bash
logsift path/to/log.ndjson
# or
cat path/to/log.ndjson | logsift
```

Each line is validated as JSON and written back out unchanged; invalid
lines are reported on stderr and dropped.

Pass `--field` to keep only lines where a field matches a value:

```bash
logsift --field level=error path/to/log.ndjson
```

`PATH` is dotted to reach nested fields, e.g. `--field meta.level=error`.
`--field` can be repeated; a line must satisfy every filter given. Values
are compared against their JSON text, so `--field code=404` matches a
numeric field too.

Pass `--since` and/or `--until` to keep only lines within a time range:

```bash
logsift --since 2024-01-01T00:00:00Z --until 2024-01-02T00:00:00Z path/to/log.ndjson
```

Both take an RFC 3339 timestamp and are inclusive bounds; either can be
given alone. By default the timestamp is read from a top-level `timestamp`
field. Use `--time-field` to point at a different (optionally dotted) path,
e.g. `--time-field meta.timestamp`. A line whose timestamp field is missing
or isn't a valid RFC 3339 string is dropped whenever `--since` or `--until`
is active. `--since`/`--until` combine with `--field` filters: a line must
satisfy all of them.

## Status

Early and under active development. Currently reads newline-delimited JSON
from a file or stdin, passes valid lines through, and supports `--field`
filtering on dotted paths and `--since`/`--until` time-range filtering. An
alternate compact output format is in progress.

## Development

```bash
cargo build
cargo test
cargo clippy --all-targets
cargo fmt --check
```
