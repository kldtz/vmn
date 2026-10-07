use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Deserialize)]
pub struct Record {
    pub byte_offset: u64,
    /// Number of bytes this record occupies in the CSV file, including its terminator.
    pub byte_len: u64,
    pub card: Card,
}

#[derive(Clone, Deserialize, Serialize, FieldNamesAsArray)]
pub struct Card {
    pub front: String,
    pub back: String,
    pub last_review: NaiveDate,
    pub next_review: NaiveDate,
}
