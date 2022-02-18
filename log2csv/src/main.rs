use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::fs;
use std::str::FromStr;

fn main() -> Result<(), String> {
    let stdin = fs::read_to_string("/dev/stdin").unwrap();
    let commits = stdin
        .split(0 as char)
        .filter(|l| !l.is_empty())
        .map(str::parse::<Commit>)
        .collect::<Result<Vec<Commit>, _>>()?;
    for commit in commits.iter().take(10) {
        println!("{:#?}", commit);
        // println!(
        //     "{:?} {:?} {:?}\n",
        //     changes.captures(line),
        //     inserts.captures(line),
        //     deletions.captures(line),
        // );
    }
    Ok(())
}

#[derive(Default, Debug, Clone)]
struct Commit {
    hash:          String,
    author_date:   String,
    author_email:  String,
    commit_date:   String,
    files_changed: u64,
    insertions:    u64,
    deletions:     u64,
}

impl FromStr for Commit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (hash, s) =
            s.split_once(' ').ok_or("parsing commit hash failed")?;
        let (author_date, s) =
            s.split_once(' ').ok_or("parsing author date failed")?;
        let (author_email, s) =
            s.split_once(' ').ok_or("parsing author email failed")?;
        let (commit_date, s) =
            s.split_once(' ').ok_or("parsing commit date failed")?;
        lazy_static! {
            static ref re_changes: Regex =
                Regex::new(r"(\d+) files changed").unwrap();
            static ref re_insertions: Regex =
                Regex::new(r"(\d+) insertions").unwrap();
            static ref re_deletions: Regex =
                Regex::new(r"(\d+) deletions").unwrap();
        }
        let files_changed = parse_regex_capture(re_changes.captures(s));
        let insertions = parse_regex_capture(re_insertions.captures(s));
        let deletions = parse_regex_capture(re_deletions.captures(s));
        Ok(Self {
            hash: hash.trim().to_string(),
            author_date: author_date.trim().to_string(),
            commit_date: commit_date.trim().to_string(),
            author_email: author_email.trim().to_string(),
            files_changed,
            insertions,
            deletions,
        })
    }
}

fn parse_regex_capture(c: Option<Captures>) -> u64 {
    if let Some(c) = c {
        if let Some(m) = c.get(1) {
            m.as_str().parse::<u64>().unwrap()
        } else {
            0
        }
    } else {
        0
    }
}
