use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, BufRead, Write};
use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDate};

use crate::models::Card;
use crate::utils::{create_reader, print, read_line, NoopWriter};

/// Lets user add as many new cards as he wants to a given CSV file.
pub fn add(path: &Path, silent: bool) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!(
            "File {:?} doesn't exist. Use `vmn init` to create it. Aborting.",
            path
        ));
    }
    let mut stdout_lock: Box<dyn Write> = if silent {
        Box::new(NoopWriter {})
    } else {
        Box::new(stdout().lock())
    };
    let (fronts, next_line) = build_lookup_table(path, &mut stdout_lock)?;
    let mut stdin_lock = stdin().lock();
    let file = OpenOptions::new().append(true).open(path)?;
    let now = Local::now().date_naive();
    add_cards(
        now,
        file,
        &mut stdin_lock,
        &mut stdout_lock,
        fronts,
        next_line,
    )
}

fn add_cards<F, R, W>(
    now: NaiveDate,
    file: F,
    mut stdin: R,
    mut stdout: W,
    mut fronts: HashMap<String, usize>,
    mut next_line: usize,
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
        print(&mut stdout, b"Front: ")?;
        let front: String = read_line(&mut stdin)?;
        // Exit on empty input
        if front.is_empty() {
            return Ok(());
        }

        let mut skip = false;

        if let Some(i) = fronts.get(&front) {
            skip = true;
            writeln!(&mut stdout, "A card with this front side already exists. Please check line {} of your CSV file!", i)?;
        }

        print(&mut stdout, b"Back:  ")?;
        let back: String = read_line(&mut stdin)?;
        print(&mut stdout, b"\n")?;
        if skip {
            continue;
        }
        writer.serialize(Card {
            front: front.clone(),
            back,
            last_review: now,
            next_review: now,
        })?;
        writer.flush()?;
        fronts.insert(front, next_line);
        next_line += 1;
    }
}

/// Builds a front -> line lookup table and returns it together with the line number
/// the next appended card will occupy.
fn build_lookup_table<W: Write>(
    path: &Path,
    mut stdout: W,
) -> Result<(HashMap<String, usize>, usize)> {
    let mut reader = create_reader(path)?;
    let mut fronts = HashMap::<String, usize>::new();
    let mut line = 1;
    for record in reader.records() {
        line += 1;
        let card = record?.deserialize::<Card>(None)?;
        if let Some(j) = fronts.get(&card.front) {
            writeln!(&mut stdout, "The front side {} in line {} is a duplicate! Please check line {} of your CSV file!", &card.front, line, j)?;
        }
        fronts.insert(card.front, line);
    }
    Ok((fronts, line + 1))
}

#[test]
fn test_add_cards_detects_and_skips_duplicate() {
    use std::io::Cursor;

    let mut file = Cursor::new(Vec::new());
    let mut stdout = Cursor::new(Vec::new());
    let mut stdin = Cursor::new(
        b"a\nb\n\
    c\nd\n\
    a\nf\n\
    f\ng\n",
    );
    let date = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let result = add_cards(date, &mut file, &mut stdin, &mut stdout, HashMap::new(), 2);
    assert!(result.is_ok());

    // Check prompts
    let stdout_vec = stdout.into_inner();
    assert_eq!(
        String::from_utf8_lossy(&stdout_vec),
        "Front: Back:  \nFront: Back:  \nFront: A card with this front side already exists. Please check line 2 of your CSV file!\nBack:  \nFront: Back:  \nFront: "
    );

    // Check output written to CSV file
    let output_vec = file.into_inner();
    let output = String::from_utf8_lossy(&output_vec);
    assert_eq!(
        output,
        "a|b|2025-05-10|2025-05-10\n\
    c|d|2025-05-10|2025-05-10\n\
    f|g|2025-05-10|2025-05-10\n"
    );
}
