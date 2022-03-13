# git-to-json

Convert git commit data from `git log` lines to json/csv/postgres.

## Usage

```bash
cargo install --path ./git2json
git2json . json
```

In general
```bash
git2json /path/to/repository json > commits.json
git2json /path/to/repository csv > commits.csv
git2json /path/to/repository postgres > create_commits.sql
```
