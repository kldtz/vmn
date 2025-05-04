use crate::models::Card;
use crate::utils::{create_reader, parse_timespan};
use anyhow::{anyhow, Result};
use chrono::{Local, TimeDelta};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::path::Path;
use std::process::exit;
use text_io::read;

/// Lets user add as many new cards as he wants to a given CSV file.
pub fn add(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!(
            "File {:?} doesn't exist. Use `vmn init` to create it. Aborting.",
            path
        ));
    }

    let (forward, backward) = build_lookup_tables(path)?;

    let now = Local::now().date_naive();
    let file = OpenOptions::new().append(true).open(path)?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .has_headers(false)
        .from_writer(file);

    let mut lock = stdout().lock();

    loop {
        write!(lock, "Front: ")?;
        let front: String = read!("{}\n");
        if let Some(i) = forward.get(&front) {
            writeln!(
                lock,
                "A card with this front side already exists. Please check line {} of {:?}!",
                i, path
            )?;
            exit(1);
        }

        write!(lock, "Back:  ")?;
        let back: String = read!("{}\n");
        if let Some(i) = backward.get(&front) {
            writeln!(
                lock,
                "A card with this back side already exists. Please check line {} of {:?}!",
                i, path
            )?;
            exit(1);
        }

        write!(lock, "Days:  ")?;
        let next_review: String = read!("{}\n");
        let timedelta = if next_review.is_empty() {
            Ok(TimeDelta::days(0))
        } else {
            parse_timespan(&next_review)
        }?;
        writeln!(lock)?;
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
