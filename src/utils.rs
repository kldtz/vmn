use anyhow::Result;
use chrono::TimeDelta;
use csv::Reader;
use std::fs::File;
use std::io;
use std::io::{StdoutLock, Write};
use std::path::Path;

pub fn parse_timespan(s: &str) -> Result<TimeDelta> {
    let days: i64 = s.parse()?;
    Ok(TimeDelta::days(days))
}

pub fn clear(lock: &mut StdoutLock) -> io::Result<()> {
    write!(lock, "{esc}[2J{esc}[1;1H", esc = 27 as char)
}

pub fn create_reader(path: &Path) -> csv::Result<Reader<File>> {
    csv::ReaderBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .has_headers(true)
        .from_path(path)
}
