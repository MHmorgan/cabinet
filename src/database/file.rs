//! Interface for file entries in the database.

use crate::file::{File, NewFile};
use crate::{CabinetError, CabinetResult as Result};
use rusqlite::Connection;
use std::convert::TryFrom;
use std::path::Path;

pub async fn exists(conn: &Connection, ident: FileIdentifier<'_>) -> Result<bool> {
    let mut exists = false;
    if let Some(id) = ident.get_id(conn).await? {
        exists = conn
            .prepare("SELECT name FROM file WHERE id IS ?")?
            .exists(&[&id])?
    }
    Ok(exists)
}

pub async fn fetch(conn: &Connection, ident: FileIdentifier<'_>) -> Result<File> {
    let id = ident.get_id(conn).await?;
    if id.is_none() {
        return Err(CabinetError::NotFound);
    }
    let file = conn
        .prepare(
            "SELECT file.id, path, content, mode, modified
            FROM file JOIN file_path ON file.id=file_path.id
            WHERE file.id IS ?",
        )?
        .query_row(&[&id], |row| File::try_from(row))?;
    Ok(file)
}

pub async fn create(conn: &Connection, file: &NewFile) -> Result<()> {
    use crate::database::dir;

    //
    // Get parent id. Creating all parents if they don't exist.
    //
    let path = Path::new(&file.path);
    let name = path.file_name().map(|s| s.to_string_lossy());
    if name.is_none() {
        return_error!("Unable to get file name from path: {}", file.path);
    }
    let empty_parents = &[Path::new(""), Path::new("/")];
    let parent = match path.parent() {
        Some(path) if !empty_parents.contains(&path) => {
            match dir::get_id(conn, path).await? {
                Some(id) => Some(id),
                None => Some(dir::create(conn, &path).await?)
            }
        }
        _ => None,
    };

    //
    // Create the new file.
    //
    let mut stmt = conn.prepare(
        "INSERT INTO file(name, parent, content, mode, modified) VALUES (?, ?, ?, ?, ?)",
    )?;
    stmt.insert(params![
        name,
        parent,
        file.content,
        file.mode,
        file.modified,
    ])?;

    Ok(())
}

pub async fn update(conn: &Connection, file: &File) -> Result<()> {
    use crate::database::dir;

    //
    // Get parent id. Creating all parents if they don't exist.
    //
    let path = Path::new(&file.path);
    let name = path.file_name().map(|s| s.to_string_lossy());
    if name.is_none() {
        return_error!("Unable to get file name from path: {}", file.path);
    }
    let empty_parents = &[Path::new(""), Path::new("/")];
    let parent = match path.parent() {
        Some(path) if !empty_parents.contains(&path) => {
            match dir::get_id(conn, path).await? {
                Some(id) => Some(id),
                None => Some(dir::create(conn, &path).await?)
            }
        }
        _ => None,
    };

    //
    // Update the file entry.
    //
    let mut stmt = conn.prepare(
        "UPDATE file SET name=?, parent=?, content=?, mode=?, modified=? WHERE id IS ?"
    )?;
    stmt.insert(params![
        name,
        parent,
        file.content,
        file.mode,
        file.modified,
        file.id,
    ])?;

    Ok(())
}

pub async fn delete(conn: &Connection, ident: FileIdentifier<'_>) -> Result<usize> {
    let id = ident.get_id(conn).await?;
    if id.is_none() {
        return Err(CabinetError::NotFound);
    }
    let n = conn
        .prepare("DELETE FROM file WHERE id IS ?")?
        .execute(&[&id])?;
    Ok(n)
}

pub async fn get_id(conn: &Connection, path: &Path) -> Result<Option<usize>> {
    use rusqlite::OptionalExtension;
    let mut stmt = conn.prepare("SELECT id FROM file_path WHERE path IS ?")?;
    let id = stmt
        .query_row(params![path.to_string_lossy()], |row| row.get(0))
        .optional()?;
    Ok(id)
}

/*******************************************************************************
 *                                                                             *
 * File identifier
 *                                                                             *
 *******************************************************************************/

/// FileIdentifier encapsulates all the possible ways to uniquely identify
/// a file:
///
/// * Id - the unique entry ID if a single version of a file.
/// * Path - the file name and parent directory.
///
#[derive(Debug, Clone)]
pub enum FileIdentifier<'a> {
    Id(usize),
    Path(&'a Path),
}

impl FileIdentifier<'_> {
    pub async fn get_id(&self, conn: &Connection) -> Result<Option<usize>> {
        match self {
            FileIdentifier::Id(id) => Ok(Some(*id)),
            FileIdentifier::Path(path) => Ok(get_id(conn, path).await?),
        }
    }
}

/*******************************************************************************
 *                                                                             *
 * Tests
 *                                                                             *
 *******************************************************************************/

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rusqlite::Connection;

    async fn db() -> Result<Connection> {
        use crate::database::create_tables;
        let conn = Connection::open_in_memory()?;
        create_tables(&conn).await?;
        conn.execute("INSERT INTO directory VALUES (1, 'mydir', NULL)", [])?;
        Ok(conn)
    }

    #[async_std::test]
    async fn all() -> Result<()> {
        let conn = db().await?;
        let path_ident1 = FileIdentifier::Path("mydir/myfile".as_ref());
        let path_ident2 = FileIdentifier::Path("foo.txt".as_ref());

        assert!(!exists(&conn, path_ident1.clone()).await.unwrap());
        create(
            &conn,
            &NewFile {
                path: "mydir/myfile".into(),
                content: vec![104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100],
                mode: 0o755,
                modified: "Wed, 21 Oct 2015 02:22:00 GMT".to_string(),
            },
        )
        .await
        .unwrap();
        assert!(exists(&conn, path_ident1.clone()).await.unwrap());

        let mut f: File = fetch(&conn, path_ident1.clone()).await.unwrap();
        assert_eq!(f.path, "mydir/myfile".to_string());
        assert_eq!(f.mode, 0o755);

        let id_ident = FileIdentifier::Id(f.id);

        f.path = "foo.txt".to_string();
        update(&conn, &f).await.unwrap();
        let res = fetch(&conn, path_ident1.clone()).await;
        assert!(res.is_err());

        assert_eq!(f, fetch(&conn, id_ident.clone()).await.unwrap());
        assert_eq!(f, fetch(&conn, path_ident2.clone()).await.unwrap());

        delete(&conn, id_ident.clone()).await.unwrap();
        assert!(!exists(&conn, id_ident.clone()).await.unwrap());

        Ok(())
    }
}
