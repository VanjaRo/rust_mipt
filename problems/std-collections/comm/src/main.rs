#![forbid(unsafe_code)]
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

// TODO: your code goes here.

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    assert_eq!(args.len(), 3);

    let fst_path = &args[1];
    let snd_path = &args[2];

    let fst_file = File::open(fst_path).unwrap();
    let snd_file = File::open(snd_path).unwrap();

    let mut hmap = HashMap::<String, bool>::new();
    let fst_reader = BufReader::new(fst_file);
    for line in fst_reader.lines() {
        hmap.insert(line.unwrap(), false);
    }

    let snd_reader = BufReader::new(snd_file);
    for line in snd_reader.lines() {
        let s = line.unwrap();
        match hmap.get(&s) {
            Some(visited) if !(*visited) => {
                println!("{}", s);
                hmap.insert(s, true);
            }
            _ => continue,
        }
    }
}
