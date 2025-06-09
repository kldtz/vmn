use anyhow::Result;
use chrono::TimeDelta;
use csv::Reader;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

pub fn parse_timespan(s: &str) -> Result<TimeDelta> {
    let days: i64 = s.parse()?;
    Ok(TimeDelta::days(days))
}

pub fn clear<W>(lock: &mut W) -> io::Result<()>
where
    W: Write,
{
    write!(lock, "{esc}[2J{esc}[1;1H", esc = 27 as char)
}

pub fn create_reader(path: &Path) -> csv::Result<Reader<File>> {
    csv::ReaderBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .has_headers(true)
        .from_path(path)
}

pub fn read_line<R>(mut reader: R) -> io::Result<String>
where
    R: io::BufRead,
{
    let mut input = String::new();
    reader.read_line(&mut input)?;
    input.truncate(input.trim_end().len());
    Ok(input)
}

pub struct NoopWriter {}

impl Write for NoopWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}