extern crate anyhow;
extern crate async_std;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate hex;
extern crate mhlog;
extern crate mime_guess;
#[macro_use]
extern crate rusqlite;
extern crate serde;
extern crate serde_json;
extern crate sha1;
#[macro_use]
extern crate quick_error;

#[macro_use]
mod common;
mod boilerplate;
mod database;
mod dir;
mod file;
mod request_handlers;

use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use rusqlite::Connection;

const DB_NAME: &str = "cabinet.sqlite";

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    use std::str::FromStr;
    use anyhow::ensure;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let m = clap_app!(myapp =>
        (version: "2.0")
        (author: "Magnus Aa. Hirth <magnus.hirth@gmail.com>")
        (about: "Cabinet file server.")
        (@arg IP: "IP to bind server to.")
        (@arg PORT: "Port to listen on.")
        (@subcommand migrate =>
            (about: "Migrate from Cabinet v1 to v2.")
            (@arg ROOT: +required "Root of v1 file data."))
    )
    .get_matches();

    //
    // Setup database
    //
    {
        let conn = get_db_conn();
        database::create_tables(&conn).await?;
    }

    //
    // Migrate and exit if requested
    //
    if let Some(m) = m.subcommand_matches("migrate") {
        let root: &str = m.value_of("ROOT").unwrap();
        migrate(root.as_ref()).await?;
        return Ok(());
    }

    ensure!(m.is_present("IP") && m.is_present("PORT"), "Missing parameters.\n{}", m.usage());

    //
    // Run server
    //
    let ip: &str = m.value_of("IP").unwrap();
    let port: u16 = u16::from_str(m.value_of("PORT").unwrap())?;
    HttpServer::new(move || {
        App::new()
            .service(request_handlers::file::get)
            .service(request_handlers::file::head)
            .service(request_handlers::file::put)
            .service(request_handlers::file::delete)
            .service(request_handlers::dir::get)
            .service(request_handlers::dir::put)
            .service(request_handlers::dir::delete)
            .service(request_handlers::boilerplate::get_all_boilerplates)
            .service(request_handlers::boilerplate::get)
            .service(request_handlers::boilerplate::put)
            .service(request_handlers::boilerplate::delete)
            .wrap(Logger::default())
    })
    .bind((ip, port))?
    .run()
    .await?;

    Ok(())
}

pub fn get_db_conn() -> Connection {
    let conn = Connection::open(DB_NAME).expect("Opening database");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn
}

quick_error! {
    #[derive(Debug, Clone)]
    pub enum CabinetError {
        BadRequest(err: String) {}
        NotFound {}
        NotModified {}
        PreconditionFailed {}
        PailoadTooLarge {}
        InternalServerError {}
        Other(err: String) {
            from(err: actix_web::error::ParseError) -> (err.to_string())
            from(err: anyhow::Error) -> (err.to_string())
            from(err: rusqlite::Error) -> (err.to_string())
            from(err: serde_json::error::Error) -> (err.to_string())
        }
    }
}

pub type CabinetResult<T> = std::result::Result<T, CabinetError>;

/*******************************************************************************
 *                                                                             *
 * Migrate
 *                                                                             *
 *******************************************************************************/

async fn migrate(root: &std::path::Path) -> anyhow::Result<()> {
    use mhlog::{info, warn, err};
    use std::path::{Path, PathBuf};
    use crate::database::file::{create, exists};
    use crate::database::file::FileIdentifier::Path as PathId;
    use crate::file::NewFile;
    use std::fs::read;
    use std::time::SystemTime;
    use actix_web::http::header::HttpDate;
    use crate::database::boilerplate::{exists as bp_exists, create as bp_create};
    use crate::database::boilerplate::BoilerplateIdentifier::Name;
    use crate::boilerplate::NewBoilerplate;

    info!("Migrating data from {:?} to {}", root, DB_NAME);
    let mut conn = get_db_conn();
    let date = HttpDate::from(SystemTime::now());

    //
    // Find all files
    //
    fn find_files(path: &Path, thing: &str) -> Vec<PathBuf> {
        if path.is_dir() {
            info!("Looking for {} in: {:?}", thing, path);
            path.read_dir()
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    find_files(&entry.path(), thing)
                })
                .collect::<Vec<Vec<_>>>()
                .concat()
        } else {
            vec![path.into()]
        }
    }
    let files = find_files(&root.join("files"), "files");
    info!("Found {} files.", files.len());

    //
    // Migrate files
    //
    for f in files {
        if exists(&conn, PathId(&f)).await? {
            warn!("File already exists: {:?}", &f);
            continue
        }
        info!("Migrating {:?}", &f);
        let new_file = NewFile {
            path: f.to_string_lossy().into(),
            content: read(&f)?,
            mode: 0o644,
            modified: date.to_string(),
        };
        create(&conn, &new_file).await?;
    }

    //
    // Find all boilerplates
    //
    let bps = find_files(&root.join("boilerplates"), "boilerplates");
    info!("Found {} boilerplates.", bps.len());

    //
    // Migrate boilerplates
    //
    for bp in bps {
        let name: String = bp.to_string_lossy().into();
        if bp_exists(&conn, Name(&name)).await? {
            warn!("Boilerplate already exists: {:?}", &bp);
            continue
        }
        let new_bp = NewBoilerplate {
            name,
            script: None,
            files: serde_json::from_slice(&read(&bp)?)?,
        };
        bp_create(&mut conn, &new_bp).await?;
    }

    Ok(())
}
