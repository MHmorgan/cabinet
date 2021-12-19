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

mod dir;
mod file;
mod database;
mod boilerplate;
mod request_handlers;

use rusqlite::Connection;

use actix_web::middleware::Logger;
use actix_web::{
    App, HttpServer,
};

const DB_NAME: &str = "cabinet.sqlite";

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    use std::str::FromStr;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let m = clap_app!(myapp =>
        (version: "2.0")
        (author: "Magnus Aa. Hirth <magnus.hirth@gmail.com>")
        (about: "Cabinet file server.")
        (@arg IP: +required "IP to bind server to.")
        (@arg PORT: +required "Port to listen on.")
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
    // Run server
    //
    let ip: &str = m.value_of("IP").unwrap();
    let port: u16 = u16::from_str(m.value_of("PORT").unwrap())?;
    HttpServer::new(move || {
        App::new()
            .service(request_handlers::file::get)
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
        PreconditionFailed {}
        PailoadTooLarge {}
        InternalServerError {}
        Other(err: String) {
            from(err: rusqlite::Error) -> (err.to_string())
            from(err: anyhow::Error) -> (err.to_string())
            from(err: serde_json::error::Error) -> (err.to_string())
        }
    }
}

pub type CabinetResult<T> = std::result::Result<T, CabinetError>;
