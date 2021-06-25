//! Storage functionality - saving, loading, and searching for files and boilerplates.
//!

use crate::common;
use async_std::{fs, io, prelude::*};
use mime_guess::{self, Mime};
use std::path::PathBuf;
use std::time::SystemTime;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct FileEntry {
    pub name: String,
}

impl FileEntry {
    /// Create a file entry for a file.
    pub fn new<T: AsRef<str>>(fname: T) -> FileEntry {
        FileEntry {
            name: fname.as_ref().to_string(),
        }
    }

    /// Return the path of the file entity, as seen by the servers file system.
    pub fn path(&self) -> PathBuf {
        common::files_dir().join(&self.name)
    }

    /// Check if the file entity exists.
    #[inline]
    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    /// Write some text to the file. This will truncate the file, overwriting
    /// any previous content.
    pub async fn write(&self, txt: &str) -> io::Result<()> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.path())
            .await?;
        write!(&mut f, "{}", txt).await
    }

    /// Read the content of the file entity.
    pub async fn read(&self) -> io::Result<String> {
        let bytes = fs::read(self.path()).await?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Remove the file.
    pub async fn remove(&self) -> io::Result<()> {
        fs::remove_file(&self.path()).await
    }

    /// Return metadata for the file entry.
    #[inline]
    pub async fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self.path()).await
    }

    #[inline]
    pub fn is_file(&self) -> bool {
        self.path().is_file()
    }

    /// Timestamp of last modification of the file entity.
    ///
    /// May fail during IO operations, reading file metadata and checking
    /// the modified field which isn't supported on all platforms.
    #[inline]
    pub async fn modified(&self) -> io::Result<SystemTime> {
        self.metadata().await?.modified()
    }

    /// Check if the file entity has been modified since the given timestamp.
    ///
    /// If the times are equal this returns `false`.
    ///
    /// May fail during IO operations, reading file metadata and checking
    /// the modified field which isn't supported on all platforms.
    pub async fn modified_since(&self, t: SystemTime) -> io::Result<bool> {
        let modified = self.modified().await?;
        Ok(modified > t)
    }

    /// Return the MIME type of the file entity.
    ///
    /// The type is guessed from the file extension, and defaults to
    /// text/plain if the extension is unknown.
    #[inline]
    pub fn content_type(&self) -> Mime {
        mime_guess::from_path(&self.name).first_or_text_plain()
    }

    pub async fn content_hash(&self) -> io::Result<String> {
        use sha1::{Digest, Sha1};

        let content = self.read().await?;
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(hex::encode(&hash))
    }
}

impl fmt::Display for FileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
