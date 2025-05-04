use crate::add::add;
use crate::models::Card;
use anyhow::{anyhow, Result};
use std::path::Path;
use struct_field_names_as_array::FieldNamesAsArray;

/// Initializes a new CSV file.
pub fn init(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(anyhow!(
            "File {:?} already exists! Use `vmn add` to add new cards. Aborting.",
            path
        ));
    }
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'|')
        .quote(b'#')
        .has_headers(false)
        .from_path(path)?;
    writer.write_record(Card::FIELD_NAMES_AS_ARRAY)?;
    writer.flush()?;
    println!("Created new card box {:?}\n", path);
    add(path)
}
