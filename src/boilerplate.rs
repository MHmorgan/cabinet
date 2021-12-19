use crate::CabinetResult as Result;
use serde::{Deserialize, Serialize};
use rusqlite::Row;
use std::collections::HashMap;
use std::convert::TryFrom;

/// A mapping of client-side file path to server-side file path.
pub type Files = HashMap<String, String>;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Boilerplate {
    pub id: usize,
    pub name: String,
    pub modified: String,
    pub script: Option<String>,
    pub files: Files,
}

impl TryFrom<&Row<'_>> for Boilerplate {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Boilerplate, Self::Error> {
        Ok(Boilerplate {
            id: row.get("id")?,
            name: row.get("name")?,
            modified: row.get("modified")?,
            script: row.get("script")?,
            files: HashMap::new(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct NewBoilerplate {
    pub name: String,
    pub script: Option<String>,
    pub files: Files,
}

impl NewBoilerplate {
    pub fn from_json<T, B>(name: T, script: Option<String>, json: B) -> Result<Self>
    where
        T: AsRef<str>,
        B: AsRef<[u8]>
    {
        let files: Files = serde_json::from_slice(json.as_ref())?;
        let bp = NewBoilerplate {
            name: name.as_ref().to_string(),
            script: script,
            files: files,
        };
        Ok(bp)
    }
}
