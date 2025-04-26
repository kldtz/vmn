use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Deserialize)]
pub struct Record {
    pub byte_offset: u64,
    pub card: Card,
}

#[derive(Deserialize, Serialize, FieldNamesAsArray)]
pub struct Card {
    pub front: String,
    pub back: String,
    pub last_forward_review: NaiveDate,
    pub next_forward_review: NaiveDate,
    pub last_backward_review: NaiveDate,
    pub next_backward_review: NaiveDate,
}

pub struct CardRef<'a> {
    pub front: &'a str,
    pub back: &'a str,
    pub last_review: &'a mut NaiveDate,
    pub next_review: &'a mut NaiveDate,
}
