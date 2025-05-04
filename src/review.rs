use crate::models::{Card, CardRef, Record};
use crate::utils::{clear, create_reader, parse_timespan};
use anyhow::Result;
use chrono::{Local, NaiveDate, TimeDelta};
use csv::Writer;
use rand::seq::SliceRandom;
use std::cmp::max;
use std::fs::{File, OpenOptions};
use std::io::{stdout, Seek, SeekFrom, StdoutLock, Write};
use std::path::Path;
use text_io::read;

/// Lets user review all due cards until there aren't anymore.
pub fn review(path: &Path) -> Result<()> {
    let mut lock = stdout().lock();
    let now: NaiveDate = Local::now().date_naive();
    let mut records = collect_due_cards(path, now)?;
    if records.is_empty() {
        writeln!(lock, "No cards due for review in {:?}", path)?;
        return Ok(());
    }
    clear(&mut lock)?;
    writeln!(lock, "Reviewing due cards in {:?}", path)?;
    let mut rng = rand::rng();
    let file = OpenOptions::new().write(true).open(path)?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .has_headers(false)
        .from_writer(file);

    let mut num_reviews = 0;
    let mut num_cards = 0;
    let mut round = 1;
    loop {
        let mut cards_due = false;
        let mut reviews = collect_due_card_indices(&records, now);
        writeln!(
            lock,
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
        for (i, is_forward) in reviews {
            let record = &mut records[i];
            let card = &mut record.card;
            if review_card(now, is_forward, card, &mut lock)? {
                cards_due = true;
            }
            update_record(&mut writer, record)?;
            num_reviews += 1;
        }
        if !cards_due {
            break;
        }
        round += 1;
    }

    writeln!(
        lock,
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
    let records: Vec<Record> = reader
        .records()
        .map(|r| {
            let r = r?;
            Ok(Record {
                byte_offset: r.position().unwrap().byte(),
                card: r.deserialize::<Card>(None)?,
            })
        })
        .filter(|r| {
            if let Ok(r) = r {
                r.card.next_forward_review <= now || r.card.next_backward_review <= now
            } else {
                false
            }
        })
        .collect::<Result<Vec<_>, csv::Error>>()?;
    Ok(records)
}

fn collect_due_card_indices(cards: &[Record], now: NaiveDate) -> Vec<(usize, bool)> {
    // Find two sets of indexes: due forward reviews & due backward reviews
    let mut reviews: Vec<(usize, bool)> = Vec::new();
    for (i, record) in cards.iter().enumerate() {
        if record.card.next_forward_review <= now {
            reviews.push((i, true));
        }
        if record.card.next_backward_review <= now {
            reviews.push((i, false));
        }
    }
    reviews
}

// Lets user review card. Returns true if the card is rescheduled for review on the same day.
fn review_card(
    now: NaiveDate,
    is_forward: bool,
    card: &mut Card,
    lock: &mut StdoutLock,
) -> Result<bool> {
    let card_ref = if is_forward {
        CardRef {
            front: &card.front,
            back: &card.back,
            last_review: &mut card.last_forward_review,
            next_review: &mut card.next_forward_review,
        }
    } else {
        CardRef {
            front: &card.back,
            back: &card.front,
            last_review: &mut card.last_backward_review,
            next_review: &mut card.next_backward_review,
        }
    };

    write!(lock, "F: {}", card_ref.front)?;
    let _: String = read!("{}\n");

    let num_days = (now - *card_ref.last_review).num_days();
    let suffix = if num_days == 1 { "" } else { "s" };
    writeln!(lock, "B: {}", card_ref.back)?;
    write!(
        lock,
        "Last review: {} day{} ago. Next in: ",
        num_days, suffix
    )?;

    let timespan: String = read!("{}\n");

    let timespan: TimeDelta = if timespan.is_empty() {
        max((now - *card_ref.last_review) * 2, TimeDelta::days(1))
    } else {
        parse_timespan(&timespan)?
    };
    *card_ref.next_review = now + timespan;
    *card_ref.last_review = now;
    writeln!(lock)?;
    clear(lock)?;
    Ok(timespan.is_zero())
}

/// Replaces given record at its byte offset. Assumes that the byte size didn't change.
fn update_record(writer: &mut Writer<File>, record: &mut Record) -> Result<()> {
    writer
        .get_ref()
        .seek(SeekFrom::Start(record.byte_offset))
        .unwrap();
    writer.serialize(&record.card)?;
    writer.flush()?;
    Ok(())
}
