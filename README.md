# stack-of-git

Convert git commit data from `git log` lines to csv/json/postgres.

## Usage

```bash
cd ./log2csv
cargo build --release

./target/release/log2csv /path/to/repository csv > commits.csv
./target/release/log2csv /path/to/repository json > commits.json
./target/release/log2csv /path/to/repository postgres > create_commits.sql
```
