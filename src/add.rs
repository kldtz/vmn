use crate::models::Card;
use crate::utils::{create_reader, read_line};
use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDate};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, BufRead, Write};
use std::path::Path;

struct NoopWriter {}

impl Write for NoopWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Lets user add as many new cards as he wants to a given CSV file.
pub fn add(path: &Path, silent: bool) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!(
            "File {:?} doesn't exist. Use `vmn init` to create it. Aborting.",
            path
        ));
    }
    let (forward, backward) = build_lookup_tables(path)?;
    let mut stdout_lock: Box<dyn Write> = if silent {
        Box::new(NoopWriter {})
    } else {
        Box::new(stdout().lock())
    };
    let mut stdin_lock = stdin().lock();
    let file = OpenOptions::new().append(true).open(path)?;
    let now = Local::now().date_naive();
    add_cards(
        now,
        file,
        &mut stdin_lock,
        &mut stdout_lock,
        (forward, backward),
    )
}

fn add_cards<F, R, W>(
    now: NaiveDate,
    file: F,
    mut stdin: R,
    mut stdout: W,
    (mut forward, mut backward): (HashMap<String, usize>, HashMap<String, usize>),
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
        // Exit on empty input
        if front.is_empty() {
            return Ok(());
        }

        if let Some(i) = forward.get(&front) {
            return Err(anyhow!(
                "A card with this front side already exists. Please check line {} of your CSV file!",
                i
            ));
        }

        stdout.write_all(b"Back:  ")?;
        stdout.flush()?;
        let back: String = read_line(&mut stdin)?;
        if let Some(i) = backward.get(&back) {
            return Err(anyhow!(
                "A card with this back side already exists. Please check line {} of your CSV file!",
                i,
            ));
        }
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        writer.serialize(Card {
            front: front.clone(),
            back: back.clone(),
            last_forward_review: now,
            next_forward_review: now,
            last_backward_review: now,
            next_backward_review: now,
        })?;
        writer.flush()?;
        forward.insert(front, forward.len() + 2);
        backward.insert(back, backward.len() + 2);
    }
}

fn build_lookup_tables(path: &Path) -> Result<(HashMap<String, usize>, HashMap<String, usize>)> {
    let mut reader = create_reader(path)?;
    let mut forward = HashMap::<String, usize>::new();
    let mut backward = HashMap::<String, usize>::new();
    for (i, record) in reader.records().enumerate() {
        let line = i + 2;
        let card = record?.deserialize::<Card>(None)?;
        if let Some(j) = forward.get(&card.front) {
            return Err(anyhow!(
                "The front side {} in line {} is a duplicate! Please check line {} of your CSV file!",
                &card.front, line, j,
            ));
        }
        forward.insert(card.front, line);
        if let Some(j) = backward.get(&card.back) {
            return Err(anyhow!(
                "The back side {} in line {} is a duplicate! Please check line {} of your CSV file!",
                &card.back, line, j,
            ));
        }
        backward.insert(card.back, line);
    }
    Ok((forward, backward))
}

#[test]
fn test_add_cards_stops_at_duplicate_back() {
    use std::io::Cursor;

    let mut file = Cursor::new(Vec::new());
    let mut stdout = Cursor::new(Vec::new());
    let mut stdin = Cursor::new(
        b"a\nb\n\
    c\nd\n\
    e\nb\n",
    );
    let date = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let result = add_cards(
        date,
        &mut file,
        &mut stdin,
        &mut stdout,
        (HashMap::new(), HashMap::new()),
    );

    // Check prompts
    let stdout_vec = stdout.into_inner();
    assert_eq!(
        String::from_utf8_lossy(&stdout_vec),
        "Front: Back:  \nFront: Back:  \nFront: Back:  "
    );

    // Check result: final input is not a valid number
    assert_eq!(
        result.unwrap_err().to_string(),
        "A card with this back side already exists. Please check line 2 of your CSV file!"
    );

    // Check output written to CSV file
    let output_vec = file.into_inner();
    let output = String::from_utf8_lossy(&output_vec);
    assert_eq!(
        output,
        "a|b|2025-05-10|2025-05-10|2025-05-10|2025-05-10\n\
    c|d|2025-05-10|2025-05-10|2025-05-10|2025-05-10\n"
    );
}

#[test]
fn test_cannot_add_duplicate_front_in_same_session() {
    use std::io::Cursor;

    let mut file = Cursor::new(Vec::new());
    let mut stdout = Cursor::new(Vec::new());
    let mut stdin = Cursor::new(
        b"a\nb\n\
    a\nc\n",
    );
    let date = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let result = add_cards(
        date,
        &mut file,
        &mut stdin,
        &mut stdout,
        (HashMap::new(), HashMap::new()),
    );

    // Check prompts
    let stdout_vec = stdout.into_inner();
    assert_eq!(
        String::from_utf8_lossy(&stdout_vec),
        "Front: Back:  \nFront: "
    );

    // Check result: error message with line number
    assert_eq!(
        result.unwrap_err().to_string(),
        "A card with this front side already exists. Please check line 2 of your CSV file!"
    );

    // Check output written to CSV file
    let output_vec = file.into_inner();
    let output = String::from_utf8_lossy(&output_vec);
    assert_eq!(output, "a|b|2025-05-10|2025-05-10|2025-05-10|2025-05-10\n");
}
