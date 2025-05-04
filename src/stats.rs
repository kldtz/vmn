use crate::models::Card;
use crate::utils::create_reader;
use anyhow::Result;
use std::fmt;
use std::path::Path;

#[derive(Default)]
struct Counts {
    today: u64,
    day: u64,
    week: u64,
    month: u64,
    quarter: u64,
    year: u64,
    more: u64,
}

impl Counts {
    fn increment_count(&mut self, days: i64) {
        match days {
            0 => self.today += 1,
            1 => self.day += 1,
            2..7 => self.week += 1,
            7..30 => self.month += 1,
            30..90 => self.quarter += 1,
            90..365 => self.year += 1,
            _ => self.more += 1,
        }
    }

    fn total(&self) -> u64 {
        self.today + self.day + self.week + self.month + self.quarter + self.year + self.more
    }
}

impl fmt::Display for Counts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            concat!(
                "Latest review intervals:\n",
                "  <day     {}\n",
                "  =day     {}\n",
                "  <week    {}\n",
                "  <month   {}\n",
                "  <quarter {}\n",
                "  <year    {}\n",
                "  >=year   {}\n\n",
                "Total: {}"
            ),
            self.today,
            self.day,
            self.week,
            self.month,
            self.quarter,
            self.year,
            self.more,
            self.total(),
        )
    }
}

pub fn stats(path: &Path) -> Result<()> {
    let mut reader = create_reader(path)?;
    let mut counts = Counts::default();
    for record in reader.records() {
        let card = record?.deserialize::<Card>(None)?;
        let forward_days = (card.next_forward_review - card.last_forward_review).num_days();
        counts.increment_count(forward_days);
        let backward_days = (card.next_backward_review - card.last_backward_review).num_days();
        counts.increment_count(backward_days);
    }
    println!("{}", counts);
    Ok(())
}
