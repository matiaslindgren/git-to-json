use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::{env, fmt, path, process, str};

const USAGE: &'static str = concat!(
    "usage: extract_git_log repository_path { csv | json | postgres }",
);

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(USAGE.to_string());
    }
    let commits = collect_git_log(&args[1])?;
    let output = match args[2].as_str() {
        "csv" => Ok(commits_to_csv(&commits, ",")),
        "json" => Ok(commits_to_json(&commits)),
        "postgres" => Ok(commits_to_postgres(&commits, "git_commits")),
        x => Err(format!("unknown output specified {}", x)),
    };
    let output = output?;
    println!("{}", output);
    Ok(())
}

fn collect_git_log(repository: &str) -> Result<Vec<Commit<u32>>, String> {
    if !path::Path::new(repository).exists() {
        return Err(format!("repository '{}' does not exist", repository));
    }
    let output = process::Command::new("git")
        .arg("-C")
        .arg(repository)
        .arg("log")
        .arg("--pretty=format:%x00%H %aI %ae %cI")
        .arg("--shortstat")
        .output()
        .expect("failed to run git");
    let stdout =
        str::from_utf8(&output.stdout[..]).expect("failed to collect stdout");
    stdout
        .split(0 as char)
        .map(str::trim)
        .filter(|section| !section.is_empty())
        .map(str::parse::<Commit<u32>>)
        .collect()
}

lazy_static! {
    static ref RE_CHANGES: Regex = Regex::new(r"(\d+) files? changed").unwrap();
    static ref RE_INSERTIONS: Regex = Regex::new(r"(\d+) insertions?").unwrap();
    static ref RE_DELETIONS: Regex = Regex::new(r"(\d+) deletions?").unwrap();
    // https://stackoverflow.com/a/28022901
    static ref RE_ISO8601_TIMESTAMP: Regex = Regex::new(
r"^(?:[1-9]\d{3}-(?:(?:0[1-9]|1[0-2])-(?:0[1-9]|1\d|2[0-8])|(?:0[13-9]|1[0-2])-(?:29|30)|(?:0[13578]|1[02])-31)|(?:[1-9]\d(?:0[48]|[2468][048]|[13579][26])|(?:[2468][048]|[13579][26])00)-02-29)T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:Z|[+-][01]\d:[0-5]\d)$").unwrap();
}

#[derive(Default, Clone, fmt::Debug, serde::Serialize)]
struct Commit<DiffSize> {
    hash:          String,
    author_date:   ISO8601,
    author_email:  String,
    commit_date:   ISO8601,
    files_changed: DiffSize,
    insertions:    DiffSize,
    deletions:     DiffSize,
}

#[derive(Default, Clone, fmt::Debug)]
struct ISO8601 {
    date: String,
}

impl str::FromStr for ISO8601 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !RE_ISO8601_TIMESTAMP.is_match(&s) {
            Err(format!("date '{}' is not ISO 8601", s))
        } else {
            Ok(Self {
                date: s.to_string(),
            })
        }
    }
}

impl fmt::Display for ISO8601 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.date)
    }
}

impl serde::Serialize for ISO8601 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.date)
    }
}

impl<DiffSize> str::FromStr for Commit<DiffSize>
where
    DiffSize: str::FromStr + fmt::Display + Default + PostgresType,
    <DiffSize as str::FromStr>::Err: fmt::Debug,
{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> =
            s.splitn(5, &[' ', '\n']).map(str::trim).collect();
        let hash = parts[0].to_string();
        if hash.len() != 40 {
            return Err(format!("commit hash length {} != 40", hash.len()));
        }
        let author_date = parts[1].parse::<ISO8601>()?;
        let author_email = parts[2].to_string();
        let commit_date = parts[3].parse::<ISO8601>()?;
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
    DiffSize: str::FromStr + fmt::Display + Default + PostgresType,
    <DiffSize as str::FromStr>::Err: fmt::Debug,
{
    captures
        .and_then(|captures| captures.get(1))
        .map(|group| {
            let num = group.as_str();
            let num = num.parse::<DiffSize>();
            num.expect("failed to parse number")
        })
        .unwrap_or_default()
}

type Field = &'static str;
type Schema = Vec<(Field, Field)>;

trait PostgresType {
    fn pg_type(&self) -> Field;
}

impl PostgresType for u16 {
    fn pg_type(&self) -> Field {
        "smallint"
    }
}

impl PostgresType for u32 {
    fn pg_type(&self) -> Field {
        "integer"
    }
}

impl PostgresType for u64 {
    fn pg_type(&self) -> Field {
        "bigint"
    }
}

impl PostgresType for ISO8601 {
    fn pg_type(&self) -> Field {
        "timestamp with time zone"
    }
}

trait PostgresSchema {
    fn schema(&self) -> Schema;
    fn field_names(&self) -> Vec<&'static str>;
    fn script_create_table(&self, table_name: &str) -> String;
}

impl<DiffSize> PostgresSchema for Commit<DiffSize>
where
    DiffSize: PostgresType,
{
    fn schema(&self) -> Schema {
        vec![
            ("hash", "char(40)"),
            ("author_date", self.author_date.pg_type()),
            ("author_email", "varchar(254)"),
            ("commit_date", self.commit_date.pg_type()),
            ("files_changed", self.files_changed.pg_type()),
            ("insertions", self.insertions.pg_type()),
            ("deletions", self.deletions.pg_type()),
        ]
    }

    fn field_names(&self) -> Vec<&'static str> {
        self.schema().into_iter().map(|(name, _)| name).collect()
    }

    fn script_create_table(&self, table_name: &str) -> String {
        let (field_width, type_width) = self
            .schema()
            .iter()
            .map(|(name, pg_type)| (name.len(), pg_type.len()))
            .reduce(|(w1, w2): (usize, usize), (l1, l2): (usize, usize)| {
                (w1.max(l1), w2.max(l2))
            })
            .unwrap();
        let lines: Vec<String> = self
            .schema()
            .iter()
            .map(|&(field, pg_type)| {
                let constraint =
                    if field == "hash" { "primary key" } else { "" };
                format!(
                    "  {:<field_width$}{:<type_width$}{}",
                    field,
                    pg_type,
                    constraint,
                    field_width = field_width + 1,
                    type_width = type_width + 1,
                )
            })
            .collect();
        format!(
            "create table {} (\n{}\n);",
            table_name,
            lines[..].join(",\n")
        )
    }
}

fn commits_to_json<DiffSize>(commits: &[Commit<DiffSize>]) -> String
where
    DiffSize: serde::Serialize,
{
    serde_json::to_string_pretty(commits)
        .expect("failed converting commits to json")
}

fn commits_to_csv<DiffSize>(commits: &[Commit<DiffSize>], sep: &str) -> String
where
    DiffSize: fmt::Display + Default + PostgresType,
{
    let header = Commit::<DiffSize>::default().field_names();
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
            line[..].join(sep)
        })
        .collect();
    lines.insert(0, header[..].join(sep));
    lines.join("\n")
}

fn commits_to_postgres<DiffSize>(
    commits: &[Commit<DiffSize>],
    table_name: &str,
) -> String
where
    DiffSize: fmt::Display + Default + PostgresType,
{
    let columns: Vec<&str> = Commit::<DiffSize>::default().field_names();
    let mut lines: Vec<String> = commits
        .iter()
        .map(|c| {
            let values = format!(
                "('{}', '{}', '{}', '{}', {}, {}, {})",
                c.hash,
                c.author_date,
                c.author_email,
                c.commit_date,
                c.files_changed,
                c.insertions,
                c.deletions,
            );
            format!(
                "insert into {} ({})\n  values {};",
                table_name,
                columns[..].join(", "),
                values,
            )
        })
        .collect();
    lines.insert(
        0,
        Commit::<DiffSize>::default().script_create_table(table_name),
    );
    lines.join("\n")
}
