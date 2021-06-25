
use crate::common;
use std::path::PathBuf;
use std::fmt;
use async_std::prelude::*;
use async_std::fs;
use async_std::io;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct DirEntry {
    pub name: String,
}

impl DirEntry {
    pub fn new<T: AsRef<str>>(fname: T) -> DirEntry {
        DirEntry {
            name: fname.as_ref().to_string(),
        }
    }

    pub fn path(&self) -> PathBuf {
        common::files_dir().join(&self.name)
    }

    #[inline]
    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.path().is_dir()
    }

    pub async fn entries(&self) -> io::Result<Vec<String>> {
        let mut entries = fs::read_dir(&self.path()).await?;
        let mut names: Vec<String> = Vec::new();
        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let mut name: String = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().await?.is_dir() {
                name += "/";
            }
            names.push(name);
        }
        Ok(names)
    }

    #[inline]
    pub async fn create(&self) -> io::Result<()> {
        fs::create_dir_all(&self.path()).await
    }

    #[inline]
    pub async fn remove(&self) -> io::Result<()> {
        fs::remove_dir_all(&self.path()).await
    }
}

impl fmt::Display for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
