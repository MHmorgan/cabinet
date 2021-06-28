#[macro_use]
extern crate lazy_static;
extern crate async_std;
extern crate env_logger;
extern crate hex;
extern crate mime_guess;
extern crate serde;
extern crate serde_json;
extern crate sha1;
#[macro_use]
extern crate clap;
extern crate anyhow;

#[macro_use]
mod common;
mod boilerplate;
mod dir_entry;
mod file_entry;

use actix_web::dev::HttpResponseBuilder;
use actix_web::middleware::Logger;
use actix_web::{
    delete, get, head, put, web, App, HttpRequest, HttpResponse, HttpServer, Result as AxResult,
};
use async_std::stream::StreamExt;
use boilerplate::{all_boilerplates, dir_used_in_boilerplates, file_used_in_boilerplates, Boilerplate};
use dir_entry::DirEntry;
use file_entry::FileEntry;
use std::fs;
use std::path::PathBuf;

// Maximum allowed file size - 256kB
const MAX_SIZE: usize = 262_144;

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
    ).get_matches();

    // Server setup
    let root = PathBuf::from(m.value_of("ROOT").unwrap());
    if !root.exists() {
        fs::create_dir(&root)?;
    }
    common::init(root);
    fs::create_dir_all(&common::files_dir())?;
    fs::create_dir_all(&common::boilerplate_dir())?;

    // Run server
    let ip: &str = m.value_of("IP").unwrap();
    let port: u16 = u16::from_str(m.value_of("PORT").unwrap())?;
    HttpServer::new(|| {
        App::new()
            .service(get_file)
            .service(file_meta)
            .service(upload_file)
            .service(delete_file)
            .service(get_dir)
            .service(create_dir)
            .service(delete_dir)
            .service(get_all_boilerplates)
            .service(get_boilerplate)
            .service(upload_boilerplate)
            .service(delete_boilerplate)
            .wrap(Logger::default())
    })
    .bind((ip, port))?
    .run()
    .await?;

    Ok(())
}

/*******************************************************************************
 *                                                                             *
 * File request handlers
 *                                                                             *
 *******************************************************************************/

async fn setup_file_headers(
    entry: &FileEntry,
    mut resp: HttpResponseBuilder,
) -> AxResult<HttpResponseBuilder> {
    use actix_web::http::header::{ContentType, ETag, EntityTag, LastModified};

    let meta = entry.metadata().await?;
    let mime_type = entry.content_type();
    let etag: String = entry.content_hash().await?;

    // Prepare default response with ETAG, Last-Modified,
    // and Content-Type headers.
    resp.set(ETag(EntityTag::strong(etag.clone())));
    resp.set(LastModified(meta.modified()?.into()));
    resp.set(ContentType(mime_type));
    Ok(resp)
}

/// Handler for GET /files/<file> requests.
#[get("/files/{file:.*}")]
async fn get_file(web::Path(file): web::Path<String>, req: HttpRequest) -> AxResult<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use std::time::SystemTime;

    let headers: &HeaderMap = req.headers();
    let entry = FileEntry::new(file);
    // Return 404 if the entry doesn't exist
    if !entry.exists() {
        return Ok(not_found!("{}", &entry));
    }
    // Return 400 if the entry isn't a file
    if !entry.is_file() {
        return Ok(bad_request!("not a file: {}", &entry));
    }
    let etag: String = entry.content_hash().await?;
    let mut resp = setup_file_headers(&entry, HttpResponse::Ok()).await?;

    // If-Modified-Since condition
    if let Some(val) = headers.get("If-Modified-Since") {
        let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
        if !entry.modified_since(date).await? {
            return Ok(not_modified!(resp));
        }
    }

    // If-Unmodified-Since condition
    if let Some(val) = headers.get("If-Unmodified-Since") {
        let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
        if entry.modified_since(date).await? {
            return Ok(precondition_failed!());
        }
    }

    // If-None-Match condition
    if headers.contains_key("If-None-Match") {
        let matches: bool = headers
            .get_all("If-None-Match")
            .map(|e| e.to_str().unwrap())
            .any(|e| e == &etag);
        if !matches {
            return Ok(not_modified!(resp));
        }
    }

    Ok(resp.body(entry.read().await?))
}

/// Handler for HEAD <file> requests.
#[head("/files/{file:.*}")]
async fn file_meta(web::Path(file): web::Path<String>) -> AxResult<HttpResponse> {
    let entry = FileEntry::new(file);
    // Return 404 if the entry doesn't exist
    if !entry.exists() {
        return Ok(not_found!("{}", &entry));
    }
    let mut resp = setup_file_headers(&entry, HttpResponse::Ok()).await?;
    Ok(resp.finish())
}

/// Handler for PUT <file> requests.
#[put("/files/{file:.*}")]
async fn upload_file(
    web::Path(file): web::Path<String>,
    mut payload: web::Payload,
    req: HttpRequest,
) -> AxResult<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use std::time::SystemTime;

    let entry = FileEntry::new(file);
    let mut resp = if entry.exists() {
        HttpResponse::NoContent()
    } else {
        HttpResponse::Created()
    };
    let headers: &HeaderMap = req.headers();

    // Check modified and ETAG header conditions only if the file already exists.
    if entry.exists() {
        let etag = entry.content_hash().await?;

        // If-Unmodified-Since condition
        if let Some(val) = headers.get("If-Unmodified-Since") {
            let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
            if entry.modified_since(date).await? {
                return Ok(precondition_failed!());
            }
        }

        // If-Match condition
        if headers.contains_key("If-Match") {
            let matches: bool = headers
                .get_all("If-Match")
                .map(|e| e.to_str().unwrap())
                .any(|e| e == &etag);
            if !matches {
                return Ok(precondition_failed!());
            }
        }
    }

    // Stringify the payload
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Ok(payload_too_large!());
        }
        body.extend_from_slice(&chunk);
    }
    let body = String::from_utf8_lossy(&body);

    // Write payload to file.
    entry.write(&body).await?;
    Ok(resp.finish())
}

/// Handler for DELETE <file> requests
#[delete("/files/{file:.*}")]
async fn delete_file(
    web::Path(file): web::Path<String>,
    req: HttpRequest,
) -> AxResult<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use std::time::SystemTime;

    let entry = FileEntry::new(file);
    // Return 404 if entry doesn't exist
    if !entry.exists() {
        return Ok(not_found!());
    }
    let bps = file_used_in_boilerplates(&entry).await?;
    if bps.len() > 0 {
        let names = bps.join("\n");
        return Ok(bad_request!("file is used in boilerplates:\n{}", names));
    }
    let headers: &HeaderMap = req.headers();
    let etag = entry.content_hash().await?;

    // If-Unmodified-Since condition
    if let Some(val) = headers.get("If-Unmodified-Since") {
        let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
        if entry.modified_since(date).await? {
            return Ok(precondition_failed!());
        }
    }

    // If-Match condition
    if headers.contains_key("If-Match") {
        let matches: bool = headers
            .get_all("If-Match")
            .map(|e| e.to_str().unwrap())
            .any(|e| e == &etag);
        if !matches {
            return Ok(precondition_failed!());
        }
    }

    entry.remove().await?;
    Ok(HttpResponse::NoContent().finish())
}

/*******************************************************************************
 *                                                                             *
 * Directory request handlers
 *                                                                             *
 *******************************************************************************/

/// Get the directory content as a JSON list.
#[get("/dirs/{dir:.*}")]
async fn get_dir(web::Path(dir): web::Path<String>) -> AxResult<HttpResponse> {
    // Get directory content
    let entry = DirEntry::new(&dir);
    // Return 404 if the entry doesn't exist
    if !entry.exists() {
        return Ok(not_found!("{}", &entry));
    }
    // Return 400 if the entry isn't a directory
    if !entry.is_dir() {
        return Ok(bad_request!("not a directory {}", &entry));
    }
    let names = entry.entries().await?;

    // Create and return response object
    let mut resp = HttpResponse::Ok();
    Ok(resp.json(&names))
}

#[put("/dirs/{dir:.*}")]
async fn create_dir(web::Path(dir): web::Path<String>) -> AxResult<HttpResponse> {
    let entry = DirEntry::new(&dir);
    let mut resp = if entry.exists() {
        HttpResponse::NoContent()
    } else {
        HttpResponse::Created()
    };
    entry.create().await?;
    Ok(resp.finish())
}

#[delete("/dirs/{dir:.*}")]
async fn delete_dir(web::Path(dir): web::Path<String>) -> AxResult<HttpResponse> {
    let entry = DirEntry::new(&dir);
    // Return 404 if the entry doesn't exist
    if !entry.exists() {
        return Ok(not_found!("{}", &entry));
    }
    let bps = dir_used_in_boilerplates(&entry).await?;
    if bps.len() > 0 {
        let names = bps.join("\n");
        return Ok(bad_request!("directory is used in boilerplates:\n{}", names));
    }
    entry.remove().await?;
    Ok(HttpResponse::NoContent().finish())
}

/*******************************************************************************
 *                                                                             *
 * Boilerplate request handlers
 *                                                                             *
 *******************************************************************************/

#[get("/boilerplates")]
async fn get_all_boilerplates() -> AxResult<HttpResponse> {
    let names = all_boilerplates().await?;
    let mut resp = HttpResponse::Ok();
    Ok(resp.json(&names))
}

#[get("/boilerplates/{boilerplate:.+}")]
async fn get_boilerplate(
    web::Path(boilerplate): web::Path<String>,
    req: HttpRequest,
) -> AxResult<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::header::{ETag, EntityTag, LastModified};
    use actix_web::http::HeaderMap;
    use async_std::io::ErrorKind::NotFound;
    use std::time::SystemTime;

    let headers: &HeaderMap = req.headers();
    let bp = match Boilerplate::open(boilerplate).await {
        Ok(bp) => bp,
        Err(err) if err.kind() == NotFound => return Ok(not_found!()),
        Err(_) => return Ok(internal_server_error!()),
    };
    let etag: String = bp.content_hash().await?;
    let meta = bp.metadata().await?;
    let mut resp = HttpResponse::Ok();
    resp.set(ETag(EntityTag::strong(etag.clone())));
    resp.set(LastModified(meta.modified()?.into()));

    // If-Modified-Since condition
    if let Some(val) = headers.get("If-Modified-Since") {
        let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
        if !bp.modified_since(date).await? {
            return Ok(not_modified!(resp));
        }
    }

    // If-Unmodified-Since condition
    if let Some(val) = headers.get("If-Unmodified-Since") {
        let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
        if bp.modified_since(date).await? {
            return Ok(precondition_failed!());
        }
    }

    // If-None-Match condition
    if headers.contains_key("If-None-Match") {
        let matches: bool = headers
            .get_all("If-None-Match")
            .map(|e| e.to_str().unwrap())
            .any(|e| e == &etag);
        if !matches {
            return Ok(not_modified!(resp));
        }
    }

    Ok(resp.json(&bp.files))
}

#[put("/boilerplates/{boilerplate:.*}")]
async fn upload_boilerplate(
    web::Path(boilerplate): web::Path<String>,
    mut payload: web::Payload,
    req: HttpRequest,
) -> AxResult<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use std::time::SystemTime;

    // Get the payload
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Ok(payload_too_large!());
        }
        body.extend_from_slice(&chunk);
    }

    let bp = match Boilerplate::from_json(&boilerplate, &body) {
        Ok(bp) => bp,
        Err(err) => return Ok(bad_request!("{}", err)),
    };
    let missing = bp.missing_files();
    if missing.len() > 0 {
        let names = missing
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        return Ok(bad_request!("missing files:\n{}", names));
    }

    let mut resp = if bp.exists() {
        HttpResponse::NoContent()
    } else {
        HttpResponse::Created()
    };
    let headers: &HeaderMap = req.headers();

    // Check modified and ETAG header conditions only if the file already exists.
    if bp.exists() {
        let etag = bp.content_hash().await?;

        // If-Unmodified-Since condition
        if let Some(val) = headers.get("If-Unmodified-Since") {
            let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
            if bp.modified_since(date).await? {
                return Ok(precondition_failed!());
            }
        }

        // If-Match condition
        if headers.contains_key("If-Match") {
            let matches: bool = headers
                .get_all("If-Match")
                .map(|e| e.to_str().unwrap())
                .any(|e| e == &etag);
            if !matches {
                return Ok(precondition_failed!());
            }
        }
    }

    // Write payload to file.
    bp.write().await?;
    Ok(resp.finish())
}

#[delete("/boilerplates/{boilerplate:.*}")]
async fn delete_boilerplate(
    web::Path(boilerplate): web::Path<String>,
    req: HttpRequest,
) -> AxResult<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use async_std::io::ErrorKind::NotFound;
    use std::time::SystemTime;

    let bp = match Boilerplate::open(boilerplate).await {
        Ok(bp) => bp,
        Err(err) if err.kind() == NotFound => return Ok(not_found!()),
        Err(_) => return Ok(internal_server_error!()),
    };
    let headers: &HeaderMap = req.headers();
    let etag = bp.content_hash().await?;

    // If-Unmodified-Since condition
    if let Some(val) = headers.get("If-Unmodified-Since") {
        let date: SystemTime = val.to_str().unwrap().parse::<HttpDate>()?.into();
        if bp.modified_since(date).await? {
            return Ok(precondition_failed!());
        }
    }

    // If-Match condition
    if headers.contains_key("If-Match") {
        let matches: bool = headers
            .get_all("If-Match")
            .map(|e| e.to_str().unwrap())
            .any(|e| e == &etag);
        if !matches {
            return Ok(precondition_failed!());
        }
    }

    bp.remove().await?;
    Ok(HttpResponse::NoContent().finish())
}
