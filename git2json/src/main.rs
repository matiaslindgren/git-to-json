use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::io::BufRead;
use std::{env, fmt, io, path, process, str};

const USAGE: &'static str =
    concat!("usage: git2json repository_path { csv | json | postgres }",);

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(USAGE.to_string());
    }
    let repository = path::Path::new(&args[1]);
    if !repository.exists() {
        return Err(format!(
            "repository '{}' does not exist",
            repository.display()
        ));
    }
    let reader = spawn_log_reader(&repository)?;
    let output = match args[2].as_str() {
        "csv" => to_csv(reader),
        "json" => to_json(reader),
        "postgres" => to_postgres(reader, "commits"),
        x => return Err(format!("unknown output format '{}'", x)),
    };
    lines_to_stdout(output)
        .map_err(|err| format!("error while printing output {}", err))
}

fn spawn_log_reader(
    repository: &path::Path,
) -> Result<impl Iterator<Item = Commit>, String> {
    let stdout = process::Command::new("git")
        .arg("-C")
        .arg(repository)
        .arg("log")
        .arg("--pretty=format:%x00%H %aI %ae %cI")
        .arg("--shortstat")
        .stdout(process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to spawn git process: {}", err))?
        .stdout
        .take()
        .ok_or(format!("failed to take stdout from git process"))?;
    let reader = io::BufReader::new(stdout)
        .split(0u8)
        // maybe Iterator<Item = Result<Commit, Err>> instead of expect?
        .map(|res| res.expect("buf read error"))
        .map(|buf| String::from_utf8_lossy(&buf).trim().to_string())
        .filter(|section| !section.is_empty())
        .map(|section| {
            section.parse::<Commit>().expect("failed parsing section")
        });
    Ok(reader)
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
struct Commit {
    hash:          String,
    author_date:   ISO8601,
    author_email:  String,
    commit_date:   ISO8601,
    files_changed: u32,
    insertions:    u32,
    deletions:     u32,
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

impl str::FromStr for Commit {
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
        let files_changed = parse_regex_capture(RE_CHANGES.captures(s));
        let insertions = parse_regex_capture(RE_INSERTIONS.captures(s));
        let deletions = parse_regex_capture(RE_DELETIONS.captures(s));
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

fn parse_regex_capture(captures: Option<Captures>) -> u32 {
    captures
        .and_then(|captures| captures.get(1))
        .map(|group| {
            let num = group.as_str();
            let num = num.parse::<u32>();
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

impl PostgresSchema for Commit {
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
            "create table if not exists {} (\n{}\n);",
            table_name,
            lines[..].join(",\n")
        )
    }
}

fn to_csv<'a>(
    reader: impl Iterator<Item = Commit> + 'a,
) -> Box<dyn Iterator<Item = String> + 'a> {
    let header = Commit::default().field_names();
    let header = header[..].join(",");
    let csv_lines = reader.map(|c| {
        let line = vec![
            format!("{}", c.hash),
            format!("{}", c.author_date),
            format!("{}", c.author_email),
            format!("{}", c.commit_date),
            format!("{}", c.files_changed),
            format!("{}", c.insertions),
            format!("{}", c.deletions),
        ];
        line[..].join(",")
    });
    Box::new(vec![header].into_iter().chain(csv_lines))
}

fn to_json<'a>(
    reader: impl Iterator<Item = Commit> + 'a,
) -> Box<dyn Iterator<Item = String> + 'a> {
    Box::new(reader.map(|c| {
        serde_json::to_string(&c).expect("failed converting commits to json")
    }))
}

fn to_postgres<'a>(
    reader: impl Iterator<Item = Commit> + 'a,
    table_name: &'static str,
) -> Box<dyn Iterator<Item = String> + 'a> {
    let columns = Commit::default().field_names();
    let columns = columns[..].join(", ");
    let create_table = Commit::default().script_create_table(table_name);
    let lines_insert_into = reader.map(move |c| {
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
            "insert into {} ({}) values {};",
            table_name, columns, values,
        )
    });
    Box::new(vec![create_table].into_iter().chain(lines_insert_into))
}

fn lines_to_stdout(
    lines: impl Iterator<Item = String>,
) -> Result<(), io::Error> {
    use std::io::{ErrorKind::BrokenPipe, Write};

    let mut stdout = io::stdout();
    for line in lines {
        match writeln!(&mut stdout, "{}", line) {
            Ok(_) => (),
            Err(err) => {
                if err.kind() == BrokenPipe {
                    break;
                }
                return Err(err);
            }
        }
    }
    Ok(())
}
