use crate::boilerplate::{Boilerplate, NewBoilerplate};
use crate::{CabinetError, CabinetResult as Result};
use rusqlite::Connection;
use std::convert::TryFrom;

pub async fn count(conn: &Connection) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT count(*) FROM boilerplate")?;
    let count = stmt.query_row([], |row| row.get(0))?;
    Ok(count)
}

pub async fn all_names(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT name FROM boilerplate")?;
    let mut names = Vec::new();
    for res in stmt.query_map([], |row| row.get("name"))? {
        names.push(res?);
    }
    Ok(names)
}

pub async fn fetch(conn: &Connection, ident: BoilerplateIdentifier<'_>) -> Result<Boilerplate> {
    let id = ident.get_id(conn).await?;
    if id.is_none() {
        return Err(CabinetError::NotFound);
    }
    //
    // Boilerplate core data
    //
    let mut stmt = conn.prepare("SELECT * FROM boilerplate WHERE id IS ?")?;
    let mut bp = stmt.query_row(params![id], |row| Boilerplate::try_from(row))?;
    //
    // Boilerplate files
    //
    let mut stmt = conn.prepare("SELECT path, location FROM bp_files WHERE bp_id IS ?")?;
    let mut rows = stmt.query(&[&id])?;
    while let Some(row) = rows.next()? {
        bp.files.insert(row.get("location")?, row.get("path")?);
    }
    Ok(bp)
}

#[allow(dead_code)]
pub async fn exists(conn: &Connection, ident: BoilerplateIdentifier<'_>) -> Result<bool> {
    let id = ident.get_id(conn).await?;
    let mut found = false;
    if let Some(id) = id {
        let mut stmt = conn.prepare("SELECT id FROM boilerplate WHERE id IS ?")?;
        found = stmt.exists(&[&id])?;
    }
    Ok(found)
}

pub async fn create(conn: &mut Connection, new: &NewBoilerplate) -> Result<usize> {
    use crate::database::file::FileIdentifier::Path;
    use crate::CabinetError::BadRequest;
    use actix_web::http::header::HttpDate;
    use std::time::SystemTime;

    let date = HttpDate::from(SystemTime::now());
    let tx = conn.transaction()?;

    let bp_id: usize;
    {
        //
        // Insert boilerplate
        //
        let mut stmt =
            tx.prepare("INSERT INTO boilerplate(name, modified, script) VALUES (?, ?, ?)")?;
        stmt.insert(params![new.name, date.to_string(), new.script])?;

        //
        // Insert boilerplate files
        //
        let mut stmt = tx.prepare("SELECT id FROM boilerplate WHERE name IS ?")?;
        bp_id = stmt.query_row(params![new.name], |row| row.get(0))?;
        let mut insert_stmt =
            tx.prepare("INSERT INTO bp_file_map(boilerplate, file, location) VALUES (?, ?, ?)")?;

        for (file_path_client, file_path_server) in &new.files {
            let p = Path(file_path_server.as_ref());
            let file_id: usize = match p.get_id(&tx).await? {
                Some(file_id) => file_id,
                None => {
                    return Err(BadRequest(format!(
                        "Boilerplate references non-existing file: {}",
                        file_path_server
                    )))
                }
            };
            insert_stmt.execute(params![bp_id, file_id, file_path_client])?;
        }
    }

    tx.commit()?;
    Ok(bp_id)
}

pub async fn update(conn: &mut Connection, bp: &Boilerplate) -> Result<usize> {
    use crate::database::file::FileIdentifier::Path;
    use crate::CabinetError::BadRequest;
    use actix_web::http::header::HttpDate;
    use std::time::SystemTime;

    let date = HttpDate::from(SystemTime::now());
    let tx = conn.transaction()?;

    {
        //
        // Insert boilerplate
        //
        let mut stmt = tx.prepare(
            "UPDATE boilerplate
                SET name=?, modified=?, script=?
              WHERE id IS ?",
        )?;
        stmt.insert(params![bp.name, date.to_string(), bp.script, bp.id])?;

        //
        // Insert boilerplate files
        //
        tx.prepare("DELETE FROM bp_file_map WHERE boilerplate IS ?")?
            .execute(&[&bp.id])?;
        let mut insert_stmt =
            tx.prepare("INSERT INTO bp_file_map(boilerplate, file, location) VALUES (?, ?, ?)")?;

        for (file_path_client, file_path_server) in &bp.files {
            let p = Path(file_path_server.as_ref());
            let file_id: usize = match p.get_id(&tx).await? {
                Some(file_id) => file_id,
                None => {
                    return Err(BadRequest(format!(
                        "Boilerplate references non-existing file: {}",
                        file_path_server
                    )))
                }
            };
            insert_stmt.execute(params![bp.id, file_id, file_path_client])?;
        }
    }

    tx.commit()?;
    Ok(bp.id)
}

pub async fn delete(conn: &Connection, ident: BoilerplateIdentifier<'_>) -> Result<()> {
    let id = ident.get_id(conn).await?;
    if id.is_none() {
        return Err(CabinetError::NotFound);
    }
    conn.prepare("DELETE FROM boilerplate WHERE id IS ?")?
        .execute(&[&id])?;
    Ok(())
}

/// Get the list of all boilerplates which includes the given file.
pub async fn file_used_in_boilerplates(conn: &Connection, file_id: usize) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT boilerplate.name AS name
           FROM boilerplate JOIN bp_file_map ON boilerplate.id=bp_file_map.boilerplate
          WHERE bp_file_map.file IS ?",
    )?;
    let mut boilerplates = Vec::new();
    for res in stmt.query_map(&[&file_id], |row| row.get("name"))? {
        boilerplates.push(res?)
    }
    Ok(boilerplates)
}

pub async fn get_id(conn: &Connection, name: &str) -> Result<Option<usize>> {
    use rusqlite::OptionalExtension;
    let mut stmt = conn.prepare("SELECT id FROM boilerplate WHERE name IS ?")?;
    let id = stmt.query_row(&[name], |row| row.get(0)).optional()?;
    Ok(id)
}

/*******************************************************************************
 *                                                                             *
 * Boilerplate Identifier
 *                                                                             *
 *******************************************************************************/

#[derive(Debug, Clone)]
pub enum BoilerplateIdentifier<'a> {
    Id(usize),
    Name(&'a str),
}

impl BoilerplateIdentifier<'_> {
    async fn get_id(&self, conn: &Connection) -> Result<Option<usize>> {
        match self {
            BoilerplateIdentifier::Id(id) => Ok(Some(*id)),
            BoilerplateIdentifier::Name(name) => get_id(conn, name).await,
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
    use std::collections::HashMap;

    async fn db() -> Result<Connection> {
        use crate::database::create_tables;
        let conn = Connection::open_in_memory()?;
        create_tables(&conn).await?;
        conn.execute(
            "INSERT INTO file VALUES (1, 'myfile', NULL, ?, 493, 164123532)",
            params![Vec::new()],
        )?;
        Ok(conn)
    }

    #[async_std::test]
    async fn all_boilerplate_functions() -> Result<()> {
        let mut conn = db().await?;
        let mut files = HashMap::new();
        files.insert("myfile".to_string(), "myfile".to_string());

        let new_bp = NewBoilerplate {
            name: "Boilerplate 1".into(),
            script: None,
            files: files.clone(),
        };
        let name_ident = BoilerplateIdentifier::Name(&new_bp.name);

        assert!(!exists(&conn, name_ident.clone()).await.unwrap());
        create(&mut conn, &new_bp).await?;
        assert!(exists(&conn, name_ident.clone()).await.unwrap());

        let mut bp = fetch(&conn, name_ident.clone()).await.unwrap();
        assert_eq!(new_bp.name, bp.name);
        assert_eq!(new_bp.script, bp.script);
        assert_eq!(new_bp.files, bp.files);

        let id_ident = BoilerplateIdentifier::Id(bp.id);
        bp.name = "Updated Boilerplate".into();
        bp.script = Some("sudo apt get awesomeness".into());
        update(&mut conn, &bp).await.unwrap();
        assert_eq!(bp, fetch(&conn, id_ident.clone()).await.unwrap());

        let names = vec![bp.name];
        let res = all_names(&conn).await.unwrap();
        assert_eq!(res, names);

        let res = file_used_in_boilerplates(&conn, 1).await.unwrap();
        assert_eq!(res, names);

        Ok(())
    }
}
