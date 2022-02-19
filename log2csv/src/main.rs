use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::fmt::{Debug, Display};
use std::fs;
use std::str::FromStr;

fn main() -> Result<(), String> {
    let stdin = fs::read_to_string("/dev/stdin").unwrap();
    let commits = stdin
        .split(0 as char)
        .filter(|l| !l.trim().is_empty())
        .map(str::parse::<Commit<u32>>)
        .collect::<Result<Vec<Commit<u32>>, _>>()?;
    println!("{}", commits_to_csv(&commits, ","));
    println!("{}", commits[0].script_create_table("mytable"));
    Ok(())
}

#[derive(Default, Debug, Clone)]
struct Commit<DiffSize> {
    hash:          String,
    author_date:   String,
    author_email:  String,
    commit_date:   String,
    files_changed: DiffSize,
    insertions:    DiffSize,
    deletions:     DiffSize,
}

lazy_static! {
    static ref RE_CHANGES: Regex = Regex::new(r"(\d+) files? changed").unwrap();
    static ref RE_INSERTIONS: Regex = Regex::new(r"(\d+) insertions?").unwrap();
    static ref RE_DELETIONS: Regex = Regex::new(r"(\d+) deletions?").unwrap();
}

impl<DiffSize> FromStr for Commit<DiffSize>
where
    DiffSize: FromStr + Display + Default + PostgresNumeric,
    <DiffSize as FromStr>::Err: Debug,
{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> =
            s.splitn(5, &[' ', '\n']).map(str::trim).collect();
        let hash = parts[0].to_string();
        let author_date = parts[1].to_string();
        let author_email = parts[2].to_string();
        let commit_date = parts[3].to_string();
        let files_changed =
            parse_regex_capture::<DiffSize>(RE_CHANGES.captures(s));
        let insertions =
            parse_regex_capture::<DiffSize>(RE_INSERTIONS.captures(s));
        let deletions =
            parse_regex_capture::<DiffSize>(RE_DELETIONS.captures(s));
        Ok(Self {
            hash,
            author_date,
            commit_date,
            author_email,
            files_changed,
            insertions,
            deletions,
        })
    }
}

fn parse_regex_capture<DiffSize>(captures: Option<Captures>) -> DiffSize
where
    DiffSize: FromStr + Display + Default + PostgresNumeric,
    <DiffSize as FromStr>::Err: Debug,
{
    captures
        .and_then(|captures| captures.get(1))
        .and_then(|group| {
            let num = group.as_str();
            let num = num.parse::<DiffSize>();
            Some(num.expect("failed to parse number"))
        })
        .unwrap_or(DiffSize::default())
}

type Field = &'static str;
type Schema = Vec<(Field, Field)>;

trait PostgresNumeric {
    fn pg_type(&self) -> Field;
}

impl PostgresNumeric for u16 {
    fn pg_type(&self) -> Field {
        "smallint"
    }
}

impl PostgresNumeric for u32 {
    fn pg_type(&self) -> Field {
        "integer"
    }
}

impl PostgresNumeric for u64 {
    fn pg_type(&self) -> Field {
        "bigint"
    }
}

trait PostgresSchema {
    fn schema(&self) -> Schema;
    fn script_create_table(&self, table_name: &str) -> String;
}

impl<DiffSize> PostgresSchema for Commit<DiffSize>
where
    DiffSize: PostgresNumeric,
{
    fn schema(&self) -> Schema {
        vec![
            ("hash", "char(40)"),
            ("author_date", "timestamp"),
            ("author_email", "varchar(254)"),
            ("commit_date", "timestamp"),
            ("files_changed", self.files_changed.pg_type()),
            ("insertions", self.insertions.pg_type()),
            ("deletions", self.deletions.pg_type()),
        ]
    }

    fn script_create_table(&self, table_name: &str) -> String {
        let lines: Vec<String> = self
            .schema()
            .iter()
            .map(|&(field, pg_type)| {
                let constraint =
                    if field == "hash" { " primary key" } else { "" };
                format!("  {} {}{}", field, pg_type, constraint)
            })
            .collect();
        format!(
            "create table {} (\n{}\n);",
            table_name,
            lines[..].join(",\n")
        )
    }
}

fn commits_to_csv<DiffSize>(commits: &[Commit<DiffSize>], sep: &str) -> String
where
    DiffSize: FromStr + Display + Default + PostgresNumeric,
{
    let schema = if commits.is_empty() {
        Commit::<DiffSize>::default().schema()
    } else {
        commits.first().unwrap().schema()
    };
    let header: Vec<&str> =
        schema.iter().map(|(name, _)| name).cloned().collect();
    let header = (&header[..]).join(sep);
    let mut lines: Vec<String> = commits
        .iter()
        .map(|c| {
            let line = vec![
                format!("{}", c.hash),
                format!("{}", c.author_date),
                format!("{}", c.author_email),
                format!("{}", c.commit_date),
                format!("{}", c.files_changed),
                format!("{}", c.insertions),
                format!("{}", c.deletions),
            ];
            (&line[..]).join(sep)
        })
        .collect();
    lines.insert(0, header);
    lines.join("\n")
}
