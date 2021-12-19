use getset::Getters;
use rusqlite::Row;
use std::convert::TryFrom;
use std::path::PathBuf;

/// NewFile contains file data without any database information: no database entry
/// id or file id.
///
/// NewFile objects are used for handling incoming files from clients.
///
#[derive(Debug, Clone, Eq, PartialEq, Hash, Getters)]
#[getset(get = "pub", set = "pub")]
pub struct NewFile {
    pub path: String,
    pub content: Vec<u8>,
    pub mode: u32,
    pub modified: String,
}

/// File contains all the data of a file entry stored in the database.
///
/// File objects are used for handling files fetched from the database.
///
#[derive(Debug, Clone, Eq, PartialEq, Hash, Getters)]
#[getset(get = "pub", set = "pub")]
pub struct File {
    pub id: usize,
    pub path: String,
    pub content: Vec<u8>,
    pub mode: u32,
    pub modified: String,
}

impl File {
    #[inline]
    pub fn content_type(&self) -> mime_guess::Mime {
        mime_guess::from_path(&self.path).first_or_text_plain()
    }

    pub fn content_hash(&self) -> String {
        use sha1::{Digest, Sha1};

        let mut hasher = Sha1::new();
        hasher.update(&self.content);
        let hash = hasher.finalize();
        hex::encode(&hash)
    }
}

impl TryFrom<&Row<'_>> for File {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> Result<File, Self::Error> {
        let content: Option<Vec<u8>> = row.get("content")?;
        Ok(File {
            id: row.get("id")?,
            path: row.get("path")?,
            content: content.unwrap_or_default(),
            mode: row.get("mode")?,
            modified: row.get("modified")?,
        })
    }
}

impl std::fmt::Display for File {
    fn fmt(&self,  f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<File {} {}>", self.id, self.path)
    }
}
