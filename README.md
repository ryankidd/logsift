# logsift

A CLI for filtering structured (newline-delimited JSON) logs.

```bash
logsift path/to/log.ndjson
# or
cat path/to/log.ndjson | logsift
```

Each line is validated as JSON and written back out unchanged; invalid
lines are reported on stderr and dropped.

## Status

Early and under active development. Currently reads newline-delimited JSON
from a file or stdin and passes valid lines through. Field-based filtering,
time-range filtering, and an alternate compact output format are in
progress.

## Development

```bash
cargo build
cargo test
cargo clippy --all-targets
cargo fmt --check
```
