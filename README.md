# logsift

A small CLI for filtering newline-delimited JSON (NDJSON) logs by field
value or time range, with an optional compact output format for reading in
a terminal.

```bash
logsift --field level=error --since 2024-01-01T00:00:00Z app.ndjson
```

## Features

- Streams input line by line, so memory use stays flat regardless of file
  size — logsift never buffers the whole file.
- Filters on any field, including nested ones, via a dotted path.
- Filters on a time range read from a timestamp field.
- Prints matching lines unchanged, or as a compact single-line summary.
- Reports invalid JSON lines on stderr and drops them instead of failing
  the whole run.

## Install

### From source

```bash
git clone https://github.com/ryankidd/logsift.git
cd logsift
cargo install --path .
```

### Directly from GitHub

```bash
cargo install --git https://github.com/ryankidd/logsift.git
```

Either way, this installs a `logsift` binary to `~/.cargo/bin` (make sure
that's on your `PATH`). Requires a recent stable Rust toolchain.

## Usage

```
logsift [OPTIONS] [PATH]
```

`PATH` is a file of newline-delimited JSON. If omitted, logsift reads from
stdin. Each line is parsed as JSON and, if it passes all given filters,
written back out in the chosen `--format`. Blank lines are skipped;
invalid JSON lines are reported on stderr and dropped.

### Plain pass-through

With no options, logsift validates and passes every line through
unchanged:

```bash
logsift app.ndjson
# or
cat app.ndjson | logsift
```

```
{"timestamp":"2024-01-01T00:00:00Z","level":"info","message":"starting up","service":"api"}
{"timestamp":"2024-01-01T01:00:00Z","level":"error","message":"connection refused","service":"api"}
```

### Field filtering (`--field`)

Keep only lines where a field equals a value:

```bash
logsift --field level=error app.ndjson
```

Use a dotted path to reach a nested field, e.g. `--field meta.level=error`.
Values are compared against their JSON text, so `--field code=404` matches
a numeric field too.

`--field` can be repeated; a line must satisfy every filter given:

```bash
logsift --field level=error --field service=api app.ndjson
```

### Time-range filtering (`--since` / `--until`)

Keep only lines whose timestamp falls within a range:

```bash
logsift --since 2024-01-01T12:00:00Z --until 2024-01-02T12:00:00Z app.ndjson
```

Both flags take an RFC 3339 timestamp and are inclusive bounds; either can
be given alone. By default the timestamp is read from a top-level
`timestamp` field. Use `--time-field` to point at a different (optionally
dotted) path:

```bash
logsift --since 2024-01-01T00:00:00Z --time-field meta.timestamp app.ndjson
```

A line whose timestamp field is missing or isn't a valid RFC 3339 string
is dropped whenever `--since` or `--until` is active. Time filters combine
with `--field` filters: a line must satisfy all of them.

### Compact output (`--format compact`)

Print a single-line summary instead of the raw JSON:

```bash
logsift --format compact app.ndjson
```

```
2024-01-01T00:00:00Z info starting up
2024-01-01T01:00:00Z error connection refused
```

Compact lines are `timestamp level message`, read from the top-level
`timestamp` and `level` fields and a `message` field (falling back to
`msg`). A missing field is printed as `-`. The default format, `raw`,
prints each line unchanged. `--format` combines with `--field` and
`--since`/`--until` — it only changes how matching lines are printed.

## Development

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

CI runs all four on every push and pull request.
