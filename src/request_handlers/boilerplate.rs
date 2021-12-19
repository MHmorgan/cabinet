use crate::get_db_conn;
use crate::boilerplate::{Boilerplate, NewBoilerplate};
use actix_web::{web, HttpRequest, HttpResponse, Result};
use mhlog::err;
use std::str::FromStr;

const MAX_SIZE: usize = 262_144;

#[actix_web::get("/boilerplates")]
pub async fn get_all_boilerplates() -> Result<HttpResponse> {
    use crate::database::boilerplate::all_names;
    let conn = get_db_conn();
    let names = match all_names(&conn).await {
        Ok(names) => names,
        Err(e) => {
            err!("Failed to get all boilerplate names: {}", e);
            return Ok(internal_server_error!())
        }
    };
    let mut resp = HttpResponse::Ok();
    Ok(resp.json(&names))
}

#[actix_web::get("/boilerplates/{boilerplate:.+}")]
pub async fn get(
    web::Path(bp_name): web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::header::LastModified;
    use actix_web::http::HeaderMap;
    use crate::database::boilerplate::fetch;
    use crate::database::boilerplate::BoilerplateIdentifier::Name;
    use crate::CabinetError::NotFound;

    //
    // Prepare response
    //
    let conn = get_db_conn();
    let bp = match fetch(&conn, Name(&bp_name)).await {
        Ok(bp) => bp,
        Err(NotFound) => return Ok(not_found!("{}", &bp_name)),
        Err(e) => {
            err!("Failed to fetch boilerplate: {}", e);
            return Ok(internal_server_error!())
        }
    };
    let modified = HttpDate::from_str(&bp.modified)?;
    let mut resp = HttpResponse::Ok();
    resp.set(LastModified(modified));

    //
    // Handle request conditions
    //
    let headers: &HeaderMap = req.headers();
    if let Some(val) = headers.get("If-Modified-Since") {
        let date: HttpDate = val.to_str().unwrap().parse()?;
        if modified <= date {
            return Ok(not_modified!(resp))
        }
    }

    Ok(resp.json(&bp.files))
}

#[actix_web::put("/boilerplates/{boilerplate:.*}")]
pub async fn put(
    web::Path(boilerplate): web::Path<String>,
    mut payload: web::Payload,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use async_std::stream::StreamExt;
    use crate::database::boilerplate::{create, fetch, update};
    use crate::database::boilerplate::BoilerplateIdentifier::Name;
    use crate::CabinetError::{NotFound, BadRequest};

    //
    // Create new boilerplate object
    //
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Ok(payload_too_large!());
        }
        body.extend_from_slice(&chunk);
    }
    let bp = match NewBoilerplate::from_json(&boilerplate, None, &body) {
        Ok(bp) => bp,
        Err(err) => return Ok(bad_request!("{}", err)),
    };

    //
    // Check if the boilerplate already exists
    //
    let mut conn = get_db_conn();
    let already_exists: bool;
    let bp_entry: Option<Boilerplate>;
    match fetch(&conn, Name(&boilerplate)).await {
        Ok(bp) => {
            already_exists = true;
            bp_entry = Some(bp);
        }
        Err(NotFound) => {
            already_exists = false;
            bp_entry = None;
        }
        Err(e) => {
            err!("Failed to check if boilerplate exists: {}", e);
            return Ok(internal_server_error!());
        }
    };

    //
    // Handle request conditions
    //
    let headers: &HeaderMap = req.headers();
    if let Some(bp_entry) = &bp_entry {
        let modified = HttpDate::from_str(&bp_entry.modified)?;
        if let Some(val) = headers.get("If-Unmodified-Since") {
            let date: HttpDate = val.to_str().unwrap().parse()?;
            if modified > date {
                return Ok(precondition_failed!());
            }
        };
    }

    //
    // Create or update the boilerplate entry
    //
    let res = if let Some(mut bp_entry) = bp_entry {
        bp_entry.script = bp.script;
        bp_entry.files = bp.files;
        update(&mut conn, bp_entry).await
    } else {
        create(&mut conn, bp).await
    };
    match res {
        Err(BadRequest(txt)) => return Ok(bad_request!("{}", txt)),
        Err(e) => {
            err!("Failed to create/update boilerplate: {}", e);
            return Ok(internal_server_error!())
        }
        _ => (),
    }

    match already_exists {
        true => Ok(HttpResponse::NoContent().finish()),
        false => Ok(HttpResponse::Created().finish()),
    }
}

#[actix_web::delete("/boilerplates/{boilerplate:.*}")]
pub async fn delete(
    web::Path(bp_name): web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use actix_web::http::header::HttpDate;
    use actix_web::http::HeaderMap;
    use crate::database::boilerplate::{fetch, delete};
    use crate::database::boilerplate::BoilerplateIdentifier::{Id, Name};
    use crate::CabinetError::NotFound;

    //
    // Fetch requested boilerplate
    //
    let conn = get_db_conn();
    let bp: Boilerplate = match fetch(&conn, Name(&bp_name)).await {
        Ok(bp) => bp,
        Err(NotFound) => return Ok(not_found!("{}", &bp_name)),
        Err(e) => {
            err!("Failed to fetch boilerplate: {}", e);
            return Ok(internal_server_error!());
        }
    };

    //
    // Handle request conditions
    //
    let headers: &HeaderMap = req.headers();
    let modified = HttpDate::from_str(&bp.modified)?;
    if let Some(val) = headers.get("If-Unmodified-Since") {
        let date: HttpDate = val.to_str().unwrap().parse()?;
        if modified > date {
            return Ok(precondition_failed!());
        }
    };

    match delete(&conn, Id(bp.id)).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            err!("{}", e);
            Ok(internal_server_error!())
        }
    }
}