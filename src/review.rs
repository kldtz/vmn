use crate::models::{Card, Record};
use crate::utils::{clear, create_reader, parse_timespan, read_line};
use anyhow::{anyhow, bail, Result};
use chrono::{Local, NaiveDate, TimeDelta};
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::cmp::max;
use std::fs::{File, OpenOptions};
use std::io::{stdin, stdout, BufRead, IsTerminal, Seek, SeekFrom, Write};
use std::path::Path;
use std::str::FromStr;

/// Lets user review all due cards until there aren't anymore.
pub fn review(path: &Path) -> Result<()> {
    let clear_screen = stdout().is_terminal();
    let mut stdout_lock = stdout().lock();
    let mut stdin_lock = stdin().lock();
    let now: NaiveDate = Local::now().date_naive();
    let mut records = collect_due_cards(path, now)?;
    if records.is_empty() {
        writeln!(stdout_lock, "No cards due for review in {:?}", path)?;
        return Ok(());
    }
    if clear_screen {
        clear(&mut stdout_lock)?;
    }
    writeln!(stdout_lock, "Reviewing due cards in {:?}", path)?;
    let mut rng = rand::rng();
    let mut file = OpenOptions::new().write(true).open(path)?;

    let mut num_reviews = 0;
    let mut num_cards = 0;
    let mut round = 1;
    loop {
        let mut cards_due = false;
        let mut reviews = collect_due_card_indices(&records, now);
        writeln!(
            stdout_lock,
            "Round {}: {} card{} to review\n",
            round,
            reviews.len(),
            if reviews.len() == 1 { "" } else { "s" }
        )?;
        if num_reviews == 0 {
            num_cards = reviews.len();
        }
        reviews.shuffle(&mut rng);

        // Walk through due cards and let user review
        for i in reviews {
            let record = &mut records[i];
            if review_card(
                now,
                &mut record.card,
                &mut stdout_lock,
                &mut stdin_lock,
                &mut rng,
                clear_screen,
            )? {
                cards_due = true;
            }
            update_record(&mut file, record)?;
            num_reviews += 1;
        }
        if !cards_due {
            break;
        }
        round += 1;
    }

    writeln!(
        stdout_lock,
        "{} review{} of {} card{}. Done.",
        num_reviews,
        if num_reviews == 1 { "" } else { "s" },
        num_cards,
        if num_cards == 1 { "" } else { "s" }
    )?;
    Ok(())
}

fn collect_due_cards(path: &Path, now: NaiveDate) -> Result<Vec<Record>> {
    let mut reader = create_reader(path)?;
    let mut all: Vec<(u64, Card)> = Vec::new();
    for record in reader.records() {
        let record = record?;
        let byte_offset = record
            .position()
            .ok_or_else(|| anyhow!("record {record:?} has no byte position"))?
            .byte();
        let card = record.deserialize::<Card>(None)?;
        all.push((byte_offset, card));
    }

    // Derive each record's length from the offset of the following record, or the
    // end of the file for the last one. This lets `update_record` verify that an
    // in-place rewrite won't change the record's size.
    let file_len = std::fs::metadata(path)?.len();
    let offsets: Vec<u64> = all.iter().map(|(offset, _)| *offset).collect();
    let mut records = Vec::with_capacity(all.len());
    for (i, (byte_offset, card)) in all.into_iter().enumerate() {
        let end = offsets.get(i + 1).copied().unwrap_or(file_len);
        records.push(Record {
            byte_offset,
            byte_len: end - byte_offset,
            card,
        });
    }

    records.retain(|record| record.card.next_review <= now);
    Ok(records)
}

fn collect_due_card_indices(cards: &[Record], now: NaiveDate) -> Vec<usize> {
    let mut reviews: Vec<usize> = Vec::new();
    for (i, record) in cards.iter().enumerate() {
        if record.card.next_review <= now {
            reviews.push(i);
        }
    }
    reviews
}

// Lets user review card. Returns true if the card is rescheduled for review on the same day.
fn review_card<R, W>(
    now: NaiveDate,
    card: &mut Card,
    stdout: &mut W,
    stdin: &mut R,
    rng: &mut ThreadRng,
    clear_screen: bool,
) -> Result<bool>
where
    R: BufRead,
    W: Write,
{
    write!(stdout, "F: {}", card.front)?;
    stdout.flush()?;
    let _: String = read_line(&mut *stdin)?;

    writeln!(stdout, "B: {}", card.back)?;
    let factor = rng.random_range(2.0..3.0);
    let default_timespan = max(
        compute_interval(card.next_review, card.last_review, factor),
        TimeDelta::days(1),
    );
    write!(stdout, "Next ({}): ", default_timespan.num_days(),)?;
    stdout.flush()?;

    let next: String = read_line(&mut *stdin)?;
    let timespan: TimeDelta = if next.is_empty() {
        default_timespan
    } else if next.contains('.') {
        // parse string into float
        let factor = f64::from_str(&next)?;
        max(
            compute_interval(card.next_review, card.last_review, factor),
            TimeDelta::days(1),
        )
    } else {
        parse_timespan(&next)?
    };
    card.next_review = now + timespan;
    card.last_review = now;
    writeln!(stdout)?;
    if clear_screen {
        clear(stdout)?;
    }
    stdout.flush()?;
    Ok(timespan.is_zero())
}

fn compute_interval(next_review: NaiveDate, last_review: NaiveDate, factor: f64) -> TimeDelta {
    TimeDelta::days(((next_review - last_review).num_days() as f64 * factor).round() as i64)
}

/// Replaces given record at its byte offset, provided the serialized size is unchanged.
fn update_record(file: &mut File, record: &Record) -> Result<()> {
    // Serialize into an in-memory buffer first so the record's size can be checked
    // before touching the file. This turns silent corruption (e.g. a box with CRLF
    // line endings, which serialize one byte shorter) into an explicit error.
    let mut buf = Vec::new();
    {
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b'|')
            .quote(b'#')
            .has_headers(false)
            .from_writer(&mut buf);
        writer.serialize(&record.card)?;
        writer.flush()?;
    }
    if buf.len() as u64 != record.byte_len {
        bail!(
            "refusing to update card {:?}: serialized record is {} bytes but occupies {} bytes in \
             the file. The box likely has non-standard line endings or formatting; please normalize it.",
            record.card.front,
            buf.len(),
            record.byte_len,
        );
    }
    file.seek(SeekFrom::Start(record.byte_offset))?;
    file.write_all(&buf)?;
    file.flush()?;
    Ok(())
}

#[test]
fn test_review_card() {
    use std::io::Cursor;

    let today = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let mut card = Card {
        front: String::from("a"),
        back: String::from("b"),
        last_review: NaiveDate::from_ymd_opt(2025, 5, 8).unwrap(),
        next_review: today,
    };
    let mut stdout = Cursor::new(Vec::new());
    let mut stdin = Cursor::new(b"\n4\n");
    let result = review_card(
        today,
        &mut card,
        &mut stdout,
        &mut stdin,
        &mut rand::rng(),
        false,
    );

    // Check result: timespan is not zero
    assert!(!result.ok().unwrap());

    // Check prompts
    let stdout_vec = stdout.into_inner();
    assert!(String::from_utf8_lossy(&stdout_vec).starts_with("F: aB: b\nNext ("));

    // Check that card was updated: 4 days
    assert_eq!(card.next_review, today + TimeDelta::days(4))
}

#[test]
fn test_review_card_with_factor() {
    use std::io::Cursor;

    let today = NaiveDate::from_ymd_opt(2025, 5, 10).unwrap();
    let mut card = Card {
        front: String::from("a"),
        back: String::from("b"),
        last_review: NaiveDate::from_ymd_opt(2025, 5, 8).unwrap(),
        next_review: today,
    };
    let mut stdout = Cursor::new(Vec::new());
    let mut stdin = Cursor::new(b"\n2.5\n");
    let result = review_card(
        today,
        &mut card,
        &mut stdout,
        &mut stdin,
        &mut rand::rng(),
        false,
    );

    // Check result: timespan is not zero
    assert!(!result.ok().unwrap());

    // Check prompts
    let stdout_vec = stdout.into_inner();
    assert!(String::from_utf8_lossy(&stdout_vec).starts_with("F: aB: b\nNext ("));

    // Check that card was updated: multiply previous interval by 2.5
    assert_eq!(card.next_review, today + TimeDelta::days(5))
}

#[cfg(test)]
fn temp_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vmn_{tag}_{}_{}.csv", std::process::id(), nanos))
}

#[cfg(test)]
fn write_box(path: &Path, terminator: csv::Terminator, cards: &[Card]) {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .terminator(terminator)
        .has_headers(false)
        .from_path(path)
        .unwrap();
    writer
        .write_record(["front", "back", "last_review", "next_review"])
        .unwrap();
    for card in cards {
        writer.serialize(card).unwrap();
    }
    writer.flush().unwrap();
}

#[cfg(test)]
fn read_cards(path: &Path) -> Vec<Card> {
    let mut reader = create_reader(path).unwrap();
    reader
        .records()
        .map(|record| record.unwrap().deserialize::<Card>(None).unwrap())
        .collect()
}

#[cfg(test)]
fn far_future() -> NaiveDate {
    NaiveDate::from_ymd_opt(2999, 1, 1).unwrap()
}

#[test]
fn test_collect_due_cards_computes_spans() {
    let path = temp_path("spans");
    let first = Card {
        front: "a".into(),
        back: "b".into(),
        last_review: NaiveDate::from_ymd_opt(2025, 5, 8).unwrap(),
        next_review: NaiveDate::from_ymd_opt(2025, 5, 10).unwrap(),
    };
    let second = Card {
        front: "c".into(),
        back: "d".into(),
        last_review: NaiveDate::from_ymd_opt(2025, 5, 8).unwrap(),
        next_review: NaiveDate::from_ymd_opt(2025, 5, 10).unwrap(),
    };
    write_box(&path, csv::Terminator::Any(b'\n'), &[first, second]);

    let records = collect_due_cards(&path, far_future()).unwrap();
    assert_eq!(records.len(), 2);
    // Every span must be non-empty and the records must not overlap.
    assert!(records[0].byte_len > 0);
    assert_eq!(
        records[1].byte_offset,
        records[0].byte_offset + records[0].byte_len
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_update_record_in_place() {
    let path = temp_path("update_in_place");
    let card = Card {
        front: "a".into(),
        back: "b".into(),
        last_review: NaiveDate::from_ymd_opt(2025, 5, 8).unwrap(),
        next_review: NaiveDate::from_ymd_opt(2025, 5, 10).unwrap(),
    };
    write_box(
        &path,
        csv::Terminator::Any(b'\n'),
        std::slice::from_ref(&card),
    );

    let records = collect_due_cards(&path, far_future()).unwrap();
    assert_eq!(records.len(), 1);
    let updated = Card {
        last_review: NaiveDate::from_ymd_opt(2025, 5, 10).unwrap(),
        next_review: NaiveDate::from_ymd_opt(2025, 5, 14).unwrap(),
        ..card.clone()
    };

    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    update_record(
        &mut file,
        &Record {
            byte_offset: records[0].byte_offset,
            byte_len: records[0].byte_len,
            card: updated.clone(),
        },
    )
    .unwrap();
    drop(file);

    let reread = read_cards(&path);
    assert_eq!(reread.len(), 1);
    assert_eq!(reread[0].front, "a");
    assert_eq!(reread[0].back, "b");
    assert_eq!(reread[0].last_review, updated.last_review);
    assert_eq!(reread[0].next_review, updated.next_review);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_update_record_rejects_size_change() {
    let path = temp_path("reject_size");
    let card = Card {
        front: "a".into(),
        back: "b".into(),
        last_review: NaiveDate::from_ymd_opt(2025, 5, 8).unwrap(),
        next_review: NaiveDate::from_ymd_opt(2025, 5, 10).unwrap(),
    };
    // A CRLF box: the record span includes two terminator bytes while the canonical
    // serialization uses one, so the in-place rewrite must be refused.
    write_box(&path, csv::Terminator::CRLF, std::slice::from_ref(&card));

    let records = collect_due_cards(&path, far_future()).unwrap();
    assert_eq!(records.len(), 1);
    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    let err = update_record(&mut file, &records[0]).unwrap_err();
    assert!(err.to_string().contains("refusing to update"));

    let _ = std::fs::remove_file(&path);
}
