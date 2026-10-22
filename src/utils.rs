use anyhow::{anyhow, Result};
use chrono::TimeDelta;
use csv::Reader;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

const BOLD: &str = "\x1b[1m";
const BOLD_OFF: &str = "\x1b[22m";
const ITALIC: &str = "\x1b[3m";
const ITALIC_OFF: &str = "\x1b[23m";

/// Renders inline Markdown emphasis (`*italic*`, `**bold**` and underscore
/// variants) as ANSI escape sequences. Emphasis is not nested; unmatched
/// markers and everything else are left untouched.
pub fn render_markdown(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && chars.get(i + 1).is_some_and(|&c| matches!(c, '*' | '_')) {
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if ch != '*' && ch != '_' {
            out.push(ch);
            i += 1;
            continue;
        }

        let run = chars[i..].iter().take_while(|&&c| c == ch).count();
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + run).copied();
        let can_open = run <= 2
            && next.is_some_and(|c| !c.is_whitespace())
            && (ch != '_' || prev.is_none_or(|c| !c.is_alphanumeric()));

        if can_open {
            if let Some(close) = find_closer(&chars, i + run, ch, run) {
                let (opening, closing) = if run == 1 {
                    (ITALIC, ITALIC_OFF)
                } else {
                    (BOLD, BOLD_OFF)
                };
                out.push_str(opening);
                out.extend(&chars[i + run..close]);
                out.push_str(closing);
                i = close + run;
                continue;
            }
        }

        out.extend(&chars[i..i + run]);
        i += run;
    }

    out
}

fn find_closer(chars: &[char], mut i: usize, marker: char, len: usize) -> Option<usize> {
    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1) == Some(&marker) {
            i += 2;
            continue;
        }
        if chars[i] != marker {
            i += 1;
            continue;
        }

        let start = i;
        let run = chars[i..].iter().take_while(|&&c| c == marker).count();
        let prev = start.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(start + run).copied();
        let can_close = prev.is_some_and(|c| !c.is_whitespace())
            && (marker != '_' || next.is_none_or(|c| !c.is_alphanumeric()));
        if run == len && can_close {
            return Some(start);
        }
        i += run;
    }
    None
}

pub fn parse_timespan(s: &str) -> Result<TimeDelta> {
    let days: i64 = s.parse()?;
    if days < 0 {
        return Err(anyhow!("number of days must not be negative, got {days}"));
    }
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

pub fn print<W: Write>(mut writer: W, buf: &[u8]) -> io::Result<()> {
    writer.write_all(buf)?;
    writer.flush()
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

#[cfg(test)]
mod tests {
    use super::render_markdown;

    const BOLD: &str = "\x1b[1m";
    const BOLD_OFF: &str = "\x1b[22m";
    const ITALIC: &str = "\x1b[3m";
    const ITALIC_OFF: &str = "\x1b[23m";

    #[test]
    fn test_render_markdown_plain_text() {
        assert_eq!(render_markdown("plain text"), "plain text");
    }

    #[test]
    fn test_render_markdown_italic() {
        assert_eq!(
            render_markdown("*italic*"),
            format!("{ITALIC}italic{ITALIC_OFF}")
        );
        assert_eq!(
            render_markdown("_italic_"),
            format!("{ITALIC}italic{ITALIC_OFF}")
        );
    }

    #[test]
    fn test_render_markdown_bold() {
        assert_eq!(render_markdown("**bold**"), format!("{BOLD}bold{BOLD_OFF}"));
        assert_eq!(render_markdown("__bold__"), format!("{BOLD}bold{BOLD_OFF}"));
    }

    #[test]
    fn test_render_markdown_triple_markers_are_literal() {
        assert_eq!(render_markdown("***both***"), "***both***");
        assert_eq!(render_markdown("___both___"), "___both___");
    }

    #[test]
    fn test_render_markdown_does_not_nest() {
        assert_eq!(
            render_markdown("**bold *italic* bold**"),
            format!("{BOLD}bold *italic* bold{BOLD_OFF}")
        );
    }

    #[test]
    fn test_render_markdown_unmatched_markers_are_literal() {
        assert_eq!(render_markdown("*text"), "*text");
        assert_eq!(render_markdown("**text"), "**text");
        assert_eq!(render_markdown("***text"), "***text");
        assert_eq!(render_markdown("a * b"), "a * b");
    }

    #[test]
    fn test_render_markdown_escaped_markers() {
        assert_eq!(render_markdown(r"\*text\*"), "*text*");
        assert_eq!(render_markdown(r"\_text\_"), "_text_");
    }

    #[test]
    fn test_render_markdown_intraword_underscores() {
        assert_eq!(render_markdown("foo_bar_baz"), "foo_bar_baz");
    }
}
