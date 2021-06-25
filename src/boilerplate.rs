use crate::common;
use crate::file_entry::FileEntry;
use crate::dir_entry::DirEntry;
use async_std::fs;
use async_std::io;
use async_std::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

pub type Files = HashMap<String, String>;

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct Boilerplate {
    pub name: String,

    /// Mapping of file names (client side) to file entries (server side).
    pub files: Files,
}

impl Boilerplate {
    /// XXX: Should probably rewrite to better error handling...at some point
    pub fn from_json<T, B>(name: T, json: B) -> Result<Boilerplate, String>
    where
        T: AsRef<str>,
        B: AsRef<[u8]>
    {
        let files: Files = serde_json::from_slice(json.as_ref()).or_else(|e| Err(format!("{:?}", e)))?;
        let bp = Boilerplate {
            name: name.as_ref().to_string(),
            files: files,
        };
        if !bp.verify_name() {
            return Err(format!("invalid boilerplate name: {}", bp.name));
        }
        Ok(bp)
    }

    pub async fn open<T: AsRef<str>>(name: T) -> io::Result<Boilerplate> {
        let mut bp = Boilerplate {
            name: name.as_ref().to_string(),
            ..Default::default()
        };
        bp.files = serde_json::from_str(&bp.read_files().await?)?;
        Ok(bp)
    }

    fn verify_name(&self) -> bool {
        self.name.chars().all(|c: char| c.is_ascii_alphanumeric())
    }

    async fn read_files(&self) -> io::Result<String> {
        let mut buffer = String::new();
        fs::File::open(self.path())
            .await?
            .read_to_string(&mut buffer)
            .await?;
        Ok(buffer)
    }

    pub async fn write(&self) -> io::Result<()> {
        let data = serde_json::to_string(&self.files)?;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.path())
            .await?;
        write!(&mut f, "{}", data).await
    }

    pub async fn remove(&self) -> io::Result<()> {
        fs::remove_file(&self.path()).await
    }

    pub fn path(&self) -> PathBuf {
        common::boilerplate_dir().join(&self.name)
    }

    pub fn contains_file(&self, file: &FileEntry) -> bool {
        for name in self.files.values() {
            if name == &file.name {
                return true
            }
        }
        false
    }

    pub fn contains_dir(&self, dir: &DirEntry) -> bool {
        for name in self.files.values() {
            let name: &String = name;
            if name.starts_with(&dir.name) {
                return true
            }
        }
        false
    }

    pub fn missing_files(&self) -> Vec<FileEntry> {
        self.files
            .values()
            .map(FileEntry::new)
            .filter(|entry| !entry.exists() || !entry.is_file())
            .collect()
    }

    #[inline]
    pub async fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self.path()).await
    }

    #[inline]
    pub async fn modified(&self) -> io::Result<SystemTime> {
        self.metadata().await?.modified()
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    pub async fn modified_since(&self, t: SystemTime) -> io::Result<bool> {
        let modified = self.modified().await?;
        Ok(modified > t)
    }

    pub async fn content_hash(&self) -> io::Result<String> {
        use sha1::{Digest, Sha1};

        let content = self.read_files().await?;
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(hex::encode(&hash))
    }
}

pub async fn all_boilerplates() -> io::Result<Vec<String>> {
    let mut entries = fs::read_dir(common::boilerplate_dir()).await?;
    let mut boilerplates = Vec::new();
    while let Some(entry) = entries.next().await {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        boilerplates.push(name);
    }
    Ok(boilerplates)
}

pub async fn file_used_in_boilerplates(file: &FileEntry) -> io::Result<Vec<String>> {
    let mut bps = Vec::new();
    for name in all_boilerplates().await? {
        let bp = Boilerplate::open(&name).await?;
        if bp.contains_file(file) {
            bps.push(name);
        }
    }
    Ok(bps)
}

pub async fn dir_used_in_boilerplates(dir: &DirEntry) -> io::Result<Vec<String>> {
    let mut bps = Vec::new();
    for name in all_boilerplates().await? {
        let bp = Boilerplate::open(&name).await?;
        if bp.contains_dir(dir) {
            bps.push(name);
        }
    }
    Ok(bps)
}
