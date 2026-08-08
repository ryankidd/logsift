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

## Status

Early and under active development. Currently reads newline-delimited JSON
from a file or stdin, passes valid lines through, and supports `--field`
filtering on dotted paths. Time-range filtering and an alternate compact
output format are in progress.

## Development

```bash
cargo build
cargo test
cargo clippy --all-targets
cargo fmt --check
```
