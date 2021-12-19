use crate::dir::Directory;
use crate::file::File;
use crate::{CabinetResult as Result, CabinetError};
use rusqlite::{Connection, OptionalExtension};
use std::convert::TryFrom;
use std::path::{Component, Path};

#[derive(Debug, Clone)]
pub enum DirContent {
    Dir(Directory),
    File(File),
}

impl std::fmt::Display for DirContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DirContent::*;
        match self {
            Dir(dir) => write!(f, "{}/", dir.name),
            File(file) => {
                let name = Path::new(&file.path).file_name().unwrap().to_string_lossy();
                write!(f, "{}", name)
            }
        }
    }
}

/// Fetch a directory from the database.
pub async fn fetch(conn: &Connection, ident: DirIdentifier<'_>) -> Result<Directory> {
    let id = match ident {
        DirIdentifier::Id(id) => Some(id),
        DirIdentifier::Path(path) => get_id(conn, path).await?,
    };
    if id.is_none() {
        return Err(CabinetError::NotFound)
    }
    // let mut stmt = conn.prepare("SELECT * FROM directory WHERE id IS ?")?;
    // let dir = stmt.query_row(params![id], |row| Directory::try_from(row))?;
    let dir = query_row!(conn,
        "SELECT * FROM directory WHERE id IS ?" => |row| Directory::try_from(row);
        id
    )?;
    Ok(dir)
}

/// Check if a directory exists.
pub async fn exists(conn: &Connection, ident: DirIdentifier<'_>) -> Result<bool> {
    let id = match ident {
        DirIdentifier::Id(id) => Some(id),
        DirIdentifier::Path(path) => get_id(conn, path).await?,
    };
    let mut found = false;
    if let Some(id) = id {
        let mut stmt = conn.prepare_cached("SELECT * FROM directory WHERE id IS ?")?;
        found = stmt.exists(&[&id])?;
    }
    Ok(found)
}

/// Create a directory if it doesn't already exist.
///
/// Returns the id if the directory.
///
pub async fn create(conn: &Connection, path: &Path) -> Result<usize> {
    let mut id_stmt = conn.prepare("SELECT id FROM directory WHERE name IS ? AND parent IS ?")?;
    let mut insert_stmt = conn.prepare("INSERT INTO directory(name, parent) VALUES (?, ?)")?;
    let mut id: Option<usize> = None;
    let func = |row: &rusqlite::Row| row.get::<_, usize>("id");

    //
    // Iterate through the path components, creating the directory
    // and all its parents
    //
    for comp in path.components() {
        match comp {
            Component::Normal(name) => {
                let p = params![name.to_string_lossy(), id.clone()];
                // Check if the directory already exists
                id = id_stmt.query_row(p.clone(), func).optional()?;
                if id.is_none() {
                    // If not, create it
                    insert_stmt.execute(p)?;
                    id = Some(id_stmt.query_row(p, func)?);
                }
            }
            _ => (),
        }
    }

    if id.is_none() {
        // NOTE: Id might be None when trying to create an empty or the root directory.
        // Not an elegant error handling solution since this will result
        // in an Internal Server Error for the client...
        return_error!("Something failed when creating directory: {:?}", path);
    }
    Ok(id.unwrap())
}

/// Delete a directory.
pub async fn delete(conn: &Connection, ident: DirIdentifier<'_>) -> Result<usize> {
    let id = match ident {
        DirIdentifier::Id(id) => Some(id),
        DirIdentifier::Path(path) => get_id(conn, path).await?,
    };
    let mut n = 0;
    if id.is_some() {
        let mut stmt = conn.prepare("DELETE FROM directory WHERE id IS ?")?;
        n = stmt.execute(&[&id])?;
    }
    Ok(n)
}

/// Return the content of a directory.
pub async fn content(conn: &Connection, ident: DirIdentifier<'_>) -> Result<Vec<DirContent>> {
    if !exists(conn, ident.clone()).await? {
        return Err(CabinetError::NotFound);
    }

    let id = match ident {
        DirIdentifier::Id(id) => Some(id),
        DirIdentifier::Path(path) => get_id(conn, path).await?,
    };

    let mut dir_stmt = conn.prepare("SELECT * FROM directory WHERE parent IS ? ORDER BY name")?;
    let mut file_stmt = conn.prepare(
        "SELECT *
           FROM file JOIN file_path ON file.id=file_path.id
          WHERE parent IS ? ORDER BY name",
    )?;

    let dirs = dir_stmt.query_map(&[&id], |row| Directory::try_from(row))?;
    let files = file_stmt.query_map(&[&id], |row| File::try_from(row))?;

    let mut content = Vec::new();
    for d in dirs {
        content.push(DirContent::Dir(d?));
    }
    for f in files {
        content.push(DirContent::File(f?));
    }

    Ok(content)
}

/// Find the id of a directory by iterating through its parents.
///
/// If `None` is returned the directory doesn't exist.
///
/// TODO: Handle root directory/empty path?
pub async fn get_id(conn: &Connection, path: &Path) -> Result<Option<usize>> {
    let mut stmt = conn.prepare("SELECT id FROM directory WHERE name IS ? AND parent IS ?")?;
    let mut id: Option<usize> = None;
    for comp in path.components() {
        match comp {
            Component::Normal(name) => {
                id = stmt
                    .query_row(params![name.to_string_lossy(), id], |row| {
                        row.get::<_, usize>("id")
                    })
                    .optional()?;
            }
            _ => (),
        }
        // If id is None at this point the directory doesn't exist.
        if id.is_none() {
            break;
        }
    }
    Ok(id)
}

#[derive(Debug, Clone)]
pub enum DirIdentifier<'a> {
    Id(usize),
    Path(&'a Path),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dir::Directory;
    use anyhow::Result;
    use rusqlite::Connection;

    async fn db() -> Result<Connection> {
        use crate::database::create_tables;
        let conn = Connection::open_in_memory()?;
        create_tables(&conn).await?;
        Ok(conn)
    }

    #[async_std::test]
    async fn test_fetch() -> Result<()> {
        let conn = db().await?;
        let ident = DirIdentifier::Path("mydir/foodir".as_ref());

        //
        // Should return error before the directory is created
        //
        assert!(fetch(&conn, ident.clone()).await.is_err());

        conn.execute("INSERT INTO directory VALUES (0, 'mydir', NULL)", [])?;
        conn.execute("INSERT INTO directory VALUES (1, 'foodir', 0)", [])?;

        //
        // Should exist and have directory 0 as parent
        //
        let dir: Directory = fetch(&conn, ident).await?;
        assert_eq!(dir.parent, Some(0));

        Ok(())
    }

    #[async_std::test]
    async fn test_exists() -> Result<()> {
        let conn = db().await?;
        let ident1 = DirIdentifier::Path("mydir".as_ref());
        let ident2 = DirIdentifier::Path("mydir/foodir".as_ref());

        //
        // No directories should exist before created
        //
        assert!(!exists(&conn, ident1.clone()).await?);
        assert!(!exists(&conn, ident2.clone()).await?);

        conn.execute("INSERT INTO directory VALUES (0, 'mydir', NULL)", [])?;
        conn.execute("INSERT INTO directory VALUES (1, 'foodir', 0)", [])?;

        //
        // Should exist after created
        //
        assert!(exists(&conn, ident1).await?);
        assert!(exists(&conn, ident2).await?);

        Ok(())
    }

    #[async_std::test]
    async fn all_dir_functions() -> Result<()> {
        let conn = db().await?;
        let ident1 = DirIdentifier::Path("mydir".as_ref());
        let ident2 = DirIdentifier::Path("foodir".as_ref());
        let ident3 = DirIdentifier::Path("foodir/bardir".as_ref());

        assert!(!exists(&conn, ident1.clone()).await?);
        create(&conn, "mydir".as_ref()).await?;
        assert!(exists(&conn, ident1.clone()).await?);

        create(&conn, "foodir/bardir".as_ref()).await?;
        let d1: Directory = fetch(&conn, ident2.clone()).await?;
        let d2: Directory = fetch(&conn, ident3.clone()).await?;
        assert_eq!(d2.parent, Some(d1.id));

        create(&conn, "mydir/foodir".as_ref()).await?;
        let cont = content(&conn, ident1.clone()).await?;
        println!("mydir content: {:?}", &cont);
        assert_eq!(cont.len(), 1);

        delete(&conn, ident1.clone()).await?;
        assert!(!exists(&conn, ident1).await?);

        delete(&conn, ident2.clone()).await?;
        assert!(!exists(&conn, ident2).await?);
        let ident3 = DirIdentifier::Id(d2.id);
        assert!(!exists(&conn, ident3).await?);

        Ok(())
    }
}
