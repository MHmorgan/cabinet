
use rusqlite::Row;
use std::convert::TryFrom;

/// Directory object which must be fetched from the database.
/// 
/// Represents a directory which already exists in the database.
/// 
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Directory {
    pub id: usize,
    pub name: String,
    pub parent: Option<usize>,
}

impl TryFrom<&Row<'_>> for Directory {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> Result<Directory, Self::Error> {
        Ok(Directory {
            id: row.get("id")?,
            name: row.get("name")?,
            parent: row.get("parent")?,
        })
    }
}


