
use rusqlite::Row;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::iter::FromIterator;

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

impl Directory {
    /// Create a new directory object with the given path.
    pub fn new(id: usize, name: String, parent: Option<usize>) -> Directory {
        Directory { id, name, parent }
    }

    /// Database entry id of the directory.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Name of the directory.
    pub fn name<'a>(&'a self) -> &'a str {
        &self.name
    }

    /// Databes entry id its parent directory.
    /// 
    /// `None` means the root directory.
    /// 
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }
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


