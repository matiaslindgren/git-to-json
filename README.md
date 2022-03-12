# git to JSON

Convert git commit data from `git log` lines to json/csv/postgres.

## Usage

```bash
cd ./git2json
cargo build --release

./target/release/git2json /path/to/repository json > commits.json
./target/release/git2json /path/to/repository csv > commits.csv
./target/release/git2json /path/to/repository postgres > create_commits.sql
```
