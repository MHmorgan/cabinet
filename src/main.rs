extern crate anyhow;
#[macro_use]
extern crate async_std;
extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate env_logger;
#[macro_use]
extern crate getset;
extern crate hex;
#[macro_use]
extern crate lazy_static;
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
        (version: "1.0")
        (author: "Magnus Aa. Hirth <magnus.hirth@gmail.com>")
        (about: "Cabinet file server.")
        (@arg IP: +required "IP to bind server to.")
        (@arg PORT: +required "Port to listen on.")
        (@arg ROOT: +required "Root directory of server data.")
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

// use actix_web::dev::HttpResponseBuilder;
// use actix_web::middleware::Logger;
// use actix_web::{
//     delete, get, head, put, web, App, HttpRequest, HttpResponse, HttpServer, Result as AxResult,
// };
// use async_std::stream::StreamExt;
// use boilerplate::{
//     all_boilerplates, dir_used_in_boilerplates, file_used_in_boilerplates, Boilerplate,
// };
// use dir_entry::DirEntry;
// use file_entry::FileEntry;
// use std::fs;
// use std::path::PathBuf;
// use std::sync::RwLock;
// use std::sync::Arc;

// /*******************************************************************************
//  *                                                                             *
//  * Boilerplate request handlers
//  *                                                                             *
//  *******************************************************************************/

// #[get("/boilerplates")]
// async fn get_all_boilerplates() -> AxResult<HttpResponse> {
//     let names = all_boilerplates().await?;
//     let mut resp = HttpResponse::Ok();
//     Ok(resp.json(&names))
// }

// #[get("/boilerplates/{boilerplate:.+}")]
// async fn get_boilerplate(
//     web::Path(boilerplate): web::Path<String>,
//     req: HttpRequest,
// ) -> AxResult<HttpResponse> {
//     use actix_web::http::header::HttpDate;
//     use actix_web::http::header::{ETag, EntityTag, LastModified};
//     use actix_web::http::HeaderMap;
//     use async_std::io::ErrorKind::NotFound;
//     use std::time::SystemTime;

//     let headers: &HeaderMap = req.headers();
//     let bp = match Boilerplate::open(boilerplate).await {
//         Ok(bp) => bp,
//         Err(err) if err.kind() == NotFound => return Ok(not_found!()),
//         Err(_) => return Ok(internal_server_error!()),
//     };
//     let etag: String = bp.content_hash().await?;
//     let meta = bp.metadata().await?;
//     let mut resp = HttpResponse::Ok();
//     resp.set(ETag(EntityTag::strong(etag.clone())));
//     resp.set(LastModified(meta.modified()?.into()));

//     // If-Modified-Since condition
//     let modified_since = if let Some(val) = headers.get("If-Modified-Since") {
//         let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
//         bp.modified_since(date).await?
//     } else {
//         true
//     };
//     // If-None-Match condition
//     let none_match = if headers.contains_key("If-None-Match") {
//         let matches: bool = headers
//             .get_all("If-None-Match")
//             // ETAG values are enclosed in double quotes
//             .map(|e| e.to_str().unwrap().trim_matches('"'))
//             .any(|e| e == &etag);
//         !matches
//     } else {
//         true
//     };
//     // Only if both conditions fail will 304 Not Modified be returned
//     if !modified_since || !none_match {
//         return Ok(not_modified!(resp));
//     }

//     Ok(resp.json(&bp.files))
// }

// #[put("/boilerplates/{boilerplate:.*}")]
// async fn upload_boilerplate(
//     web::Path(boilerplate): web::Path<String>,
//     mut payload: web::Payload,
//     req: HttpRequest,
// ) -> AxResult<HttpResponse> {
//     use actix_web::http::header::HttpDate;
//     use actix_web::http::HeaderMap;
//     use std::time::SystemTime;

//     // Get the payload
//     let mut body = web::BytesMut::new();
//     while let Some(chunk) = payload.next().await {
//         let chunk = chunk?;
//         if (body.len() + chunk.len()) > MAX_SIZE {
//             return Ok(payload_too_large!());
//         }
//         body.extend_from_slice(&chunk);
//     }

//     let bp = match Boilerplate::from_json(&boilerplate, &body) {
//         Ok(bp) => bp,
//         Err(err) => return Ok(bad_request!("{}", err)),
//     };
//     let missing = bp.missing_files();
//     if missing.len() > 0 {
//         let names = missing
//             .iter()
//             .map(|e| e.to_string())
//             .collect::<Vec<String>>()
//             .join("\n");
//         return Ok(bad_request!("missing files:\n{}", names));
//     }

//     let mut resp = if bp.exists() {
//         HttpResponse::NoContent()
//     } else {
//         HttpResponse::Created()
//     };
//     let headers: &HeaderMap = req.headers();

//     // Check modified and ETAG header conditions only if the file already exists.
//     if bp.exists() {
//         let etag = bp.content_hash().await?;

//         // If-Unmodified-Since condition
//         let unmodified_since = if let Some(val) = headers.get("If-Unmodified-Since") {
//             let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
//             !bp.modified_since(date).await?
//         } else {
//             true
//         };
//         // If-Match condition
//         let if_match = if headers.contains_key("If-Match") {
//             let matches: bool = headers
//                 .get_all("If-Match")
//                 .map(|e| e.to_str().unwrap().trim_matches('"'))
//                 .any(|e| e == &etag);
//             matches
//         } else {
//             true
//         };
//         if !unmodified_since || !if_match {
//             return Ok(precondition_failed!());
//         }
//     }

//     // Write payload to file.
//     bp.write().await?;
//     Ok(resp.finish())
// }

// #[delete("/boilerplates/{boilerplate:.*}")]
// async fn delete_boilerplate(
//     web::Path(boilerplate): web::Path<String>,
//     req: HttpRequest,
// ) -> AxResult<HttpResponse> {
//     use actix_web::http::header::HttpDate;
//     use actix_web::http::HeaderMap;
//     use async_std::io::ErrorKind::NotFound;
//     use std::time::SystemTime;

//     let bp = match Boilerplate::open(boilerplate).await {
//         Ok(bp) => bp,
//         Err(err) if err.kind() == NotFound => return Ok(not_found!()),
//         Err(_) => return Ok(internal_server_error!()),
//     };
//     let headers: &HeaderMap = req.headers();
//     let etag = bp.content_hash().await?;

//     // If-Unmodified-Since condition
//     let unmodified_since = if let Some(val) = headers.get("If-Unmodified-Since") {
//         let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
//         !bp.modified_since(date).await?
//     } else {
//         true
//     };
//     // If-Match condition
//     let if_match = if headers.contains_key("If-Match") {
//         let matches: bool = headers
//             .get_all("If-Match")
//             .map(|e| e.to_str().unwrap().trim_matches('"'))
//             .any(|e| e == &etag);
//         matches
//     } else {
//         true
//     };
//     // We don't care if the file has been modified as long as its content
//     // (content hash) is unchanged. But if the content has changed
//     // the precondition (if present) always fails.
//     if !unmodified_since || !if_match {
//         return Ok(precondition_failed!());
//     }

//     bp.remove().await?;
//     Ok(HttpResponse::NoContent().finish())
// }
