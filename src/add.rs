use crate::models::Card;
use crate::utils::{create_reader, parse_timespan, read_line};
use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDate, TimeDelta};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, BufRead, Write};
use std::path::Path;

/// Lets user add as many new cards as he wants to a given CSV file.
pub fn add(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!(
            "File {:?} doesn't exist. Use `vmn init` to create it. Aborting.",
            path
        ));
    }
    let lookup_tables = build_lookup_tables(path)?;
    let mut stdout_lock = stdout().lock();
    let mut stdin_lock = stdin().lock();
    let file = OpenOptions::new().append(true).open(path)?;
    let now = Local::now().date_naive();
    add_cards(now, file, &mut stdin_lock, &mut stdout_lock, lookup_tables)
}

fn add_cards<F, R, W>(
    now: NaiveDate,
    file: F,
    mut stdin: R,
    mut stdout: W,
    (forward, backward): (HashMap<String, usize>, HashMap<String, usize>),
) -> Result<()>
where
    F: Write,
    R: BufRead,
    W: Write,
{
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .has_headers(false)
        .from_writer(file);

    loop {
        stdout.write_all(b"Front: ")?;
        stdout.flush()?;
        let front: String = read_line(&mut stdin)?;
        if let Some(i) = forward.get(&front) {
            return Err(anyhow!(
                "A card with this front side already exists. Please check line {} of your CSV file!",
                i
            ));
        }

        stdout.write_all(b"Back:  ")?;
        stdout.flush()?;
        let back: String = read_line(&mut stdin)?;
        if let Some(i) = backward.get(&front) {
            return Err(anyhow!(
                "A card with this back side already exists. Please check line {} of your CSV file!",
                i,
            ));
        }

        stdout.write_all(b"Days:  ")?;
        stdout.flush()?;
        let next_review: String = read_line(&mut stdin)?;
        let timedelta = if next_review.is_empty() {
            Ok(TimeDelta::days(0))
        } else {
            parse_timespan(&next_review)
        }?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        writer.serialize(Card {
            front,
            back,
            last_forward_review: now,
            next_forward_review: now + timedelta,
            last_backward_review: now,
            next_backward_review: now + timedelta,
        })?;
        writer.flush()?;
    }
}

fn build_lookup_tables(path: &Path) -> Result<(HashMap<String, usize>, HashMap<String, usize>)> {
    let mut reader = create_reader(path)?;
    let mut forward = HashMap::<String, usize>::new();
    let mut backward = HashMap::<String, usize>::new();
    for (i, record) in reader.records().enumerate() {
        let card = record?.deserialize::<Card>(None)?;
        forward.insert(card.front, i + 2);
        backward.insert(card.back, i + 2);
    }
    Ok((forward, backward))
}

#[test]
fn test_add_cards() {
    use std::io::Cursor;

    let mut file = Cursor::new(Vec::new());
    let mut stdout = Cursor::new(Vec::new());
    let mut stdin = Cursor::new(b"a\nb\n10\n\
    c\nd\n\n\
    e\nf\nerror");
    let lookup_tables = (HashMap::new(), HashMap::new());
    let date = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let result = add_cards(date, &mut file, &mut stdin, &mut stdout, lookup_tables);

    // Check prompts
    let stdout_vec =  stdout.into_inner();
    assert_eq!(String::from_utf8_lossy(&stdout_vec), "Front: Back:  Days:  \nFront: Back:  Days:  \nFront: Back:  Days:  ");

    // Check result: final input is not a valid number
    assert_eq!(result.unwrap_err().to_string(), "invalid digit found in string");

    // Check output written to CSV file
    let output_vec = file.into_inner();
    let output = String::from_utf8_lossy(&output_vec);
    assert_eq!(output, "a|b|2025-05-10|2025-05-20|2025-05-10|2025-05-20\n\
    c|d|2025-05-10|2025-05-10|2025-05-10|2025-05-10\n");
}
