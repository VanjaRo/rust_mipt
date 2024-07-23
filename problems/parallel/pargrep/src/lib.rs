#![forbid(unsafe_code)]

use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use rayon::prelude::*;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub path: PathBuf,
    pub line: String,
    pub line_number: usize,
}

#[derive(Debug)]
pub struct Error {
    pub path: PathBuf,
    pub error: io::Error,
}

pub enum Event {
    Match(Match),
    Error(Error),
}

pub fn run<P: AsRef<Path>>(path: P, pattern: &str) -> Vec<Event> {
    if path.as_ref().is_file() {
        match File::open(path.as_ref()) {
            Ok(file) => BufReader::new(file)
                .lines()
                .enumerate()
                .filter(|(_, line)| line.is_err() || line.as_ref().unwrap().contains(pattern))
                .map(|(line_idx, line)| match line {
                    Ok(line_content) => Event::Match(Match {
                        path: path.as_ref().to_path_buf(),
                        line: line_content,
                        line_number: line_idx + 1,
                    }),
                    Err(e) => Event::Error(Error {
                        path: path.as_ref().to_path_buf(),
                        error: e,
                    }),
                })
                .collect(),
            Err(e) => vec![Event::Error(Error {
                path: path.as_ref().to_path_buf(),
                error: e,
            })],
        }
    } else {
        match path.as_ref().read_dir() {
            Ok(rd) => rd
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<PathBuf>>()
                .par_iter()
                .map(|p| run(p, pattern))
                .flatten()
                .collect(),
            Err(e) => vec![Event::Error(Error {
                path: path.as_ref().to_path_buf(),
                error: e,
            })],
        }
    }
}
