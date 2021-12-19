use crate::{CabinetError, CabinetResult};
use crate::file::{File, NewFile};
use crate::get_db_conn;
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::header::HttpDate;
use actix_web::http::HeaderMap;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use mhlog::err;
use std::str::FromStr;

const MAX_SIZE: usize = 262_144;

async fn head_or_get(
    file_path: String,
    req: HttpRequest,
) -> CabinetResult<(HttpResponseBuilder, Vec<u8>)> {
    use crate::database::file::fetch;
    use crate::database::file::FileIdentifier::Path;
    use actix_web::http::header::{ContentType, ETag, EntityTag, LastModified};

    //
    // Fetch requested file
    //
    let conn = get_db_conn();
    let file: File = fetch(&conn, Path(file_path.as_ref())).await?;

    //
    // Prepare response header
    //
    let etag = file.content_hash();
    let mime_type = file.content_type();
    let modified = HttpDate::from_str(&file.modified)?;
    let mut resp = HttpResponse::Ok();
    resp.set(ETag(EntityTag::strong(etag.clone())));
    resp.set(LastModified(modified));
    resp.set(ContentType(mime_type));

    //
    // Handle request conditions
    //
    let headers: &HeaderMap = req.headers();
    let modified_since = if let Some(val) = headers.get("If-Modified-Since") {
        let date: HttpDate = val.to_str().unwrap().parse()?;
        modified > date
    } else {
        true
    };
    let none_match = if headers.contains_key("If-None-Match") {
        let matches: bool = headers
            .get_all("If-None-Match")
            .map(|e| e.to_str().unwrap().trim_matches('"'))
            .any(|e| e == &etag);
        !matches
    } else {
        true
    };
    if !modified_since || !none_match {
        return Err(CabinetError::NotModified);
    }

    Ok((resp, file.content))
}

#[actix_web::get("/files/{file:.*}")]
pub async fn get(
    web::Path(file_path): web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use crate::CabinetError::{NotFound, NotModified};
    let (mut resp, content) = match head_or_get(file_path.clone(), req).await {
        Ok(res) => res,
        Err(NotFound) => return Ok(not_found!("{}", &file_path)),
        Err(NotModified) => return Ok(not_modified!()),
        Err(e) => {
            err!("Failed to get file: {}", e);
            return Ok(internal_server_error!());
        }
    };
    Ok(resp.body(content))
}

#[actix_web::head("/files/{file:.*}")]
pub async fn head(
    web::Path(file_path): web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use crate::CabinetError::NotFound;
    let (mut resp, _) = match head_or_get(file_path.clone(), req).await {
        Ok(res) => res,
        Err(NotFound) => return Ok(not_found!("{}", &file_path)),
        Err(e) => {
            err!("Failed to get file (head): {}", e);
            return Ok(internal_server_error!());
        }
    };
    Ok(resp.finish())
}

#[actix_web::put("/files/{file:.*}")]
pub async fn put(
    web::Path(file_path): web::Path<String>,
    mut payload: web::Payload,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use crate::database::file::FileIdentifier::Path;
    use crate::database::file::{create, fetch, update};
    use crate::CabinetError::NotFound;
    use async_std::stream::StreamExt;
    use std::time::SystemTime;

    //
    // Prepare response
    //
    let conn = get_db_conn();
    let mut already_exists = false;
    let mut file_entry = None;
    match fetch(&conn, Path(file_path.as_ref())).await {
        Ok(f) => {
            already_exists = true;
            file_entry = Some(f);
        }
        Err(NotFound) => (),
        Err(e) => {
            err!("Unexpected error: {}", e);
            return Ok(internal_server_error!());
        }
    };
    let mut resp = if already_exists {
        HttpResponse::NoContent()
    } else {
        HttpResponse::Created()
    };

    //
    // Handle request conditions
    //
    let headers: &HeaderMap = req.headers();
    if let Some(file_entry) = &file_entry {
        let etag = file_entry.content_hash();
        let modified = match HttpDate::from_str(&file_entry.modified) {
            Ok(v) => v,
            Err(e) => {
                err!("Unexpected error: {}", e);
                return Ok(internal_server_error!());
            }
        };

        // If-Unmodified-Since condition
        let unmodified_since = if let Some(val) = headers.get("If-Unmodified-Since") {
            let date: HttpDate = val.to_str().unwrap().parse()?;
            modified <= date
        } else {
            true
        };
        // If-Match condition
        let if_match = if headers.contains_key("If-Match") {
            let matches: bool = headers
                .get_all("If-Match")
                // ETAG values are enclosed in double quotes
                .map(|e| e.to_str().unwrap().trim_matches('"'))
                .any(|e| e == &etag);
            matches
        } else {
            true
        };
        if !unmodified_since || !if_match {
            return Ok(precondition_failed!());
        }
    }

    //
    // Get payload
    //
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Ok(payload_too_large!());
        }
        body.extend_from_slice(&chunk);
    }

    //
    // Create or update the file entry
    //
    let date = HttpDate::from(SystemTime::now());
    let res = if let Some(mut file_entry) = file_entry {
        file_entry.content = Vec::from(&body[..]);
        file_entry.modified = date.to_string();
        update(&conn, &file_entry).await
    } else {
        let new_file = NewFile {
            path: file_path,
            content: Vec::from(&body[..]),
            mode: 0, // XXX: Mode are not yet implemented in the API
            modified: date.to_string(),
        };
        create(&conn, &new_file).await
    };
    if let Err(e) = res {
        err!("{}", e);
        return Ok(internal_server_error!());
    }

    Ok(resp.finish())
}

#[actix_web::delete("/files/{file:.*}")]
pub async fn delete(
    web::Path(file_path): web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    use crate::database::boilerplate::file_used_in_boilerplates;
    use crate::database::file::FileIdentifier::{Id, Path};
    use crate::database::file::{delete, fetch};
    use crate::CabinetError::NotFound;

    //
    // Fetch requested file
    //
    let conn = get_db_conn();
    let file: File = match fetch(&conn, Path(file_path.as_ref())).await {
        Ok(f) => f,
        Err(NotFound) => return Ok(not_found!("{}", &file_path)),
        Err(e) => {
            err!("Failed to fetch file: {}", e);
            return Ok(internal_server_error!());
        }
    };

    //
    // Check if the file is used in any boilerplates before deleting
    //
    let bps = match file_used_in_boilerplates(&conn, file.id).await {
        Ok(bps) => bps,
        Err(e) => {
            err!("Failed to check if file was used in boilerplates: {}", e);
            return Ok(internal_server_error!());
        }
    };
    if bps.len() > 0 {
        let names = bps.join("\n");
        return Ok(bad_request!("file is used in boilerplates:\n{}", names));
    }

    //
    // Handle request conditions
    //
    let headers: &HeaderMap = req.headers();
    let etag = file.content_hash();
    let modified = HttpDate::from_str(&file.modified)?;
    let unmodified_since = if let Some(val) = headers.get("If-Unmodified-Since") {
        let date = val.to_str().unwrap().parse()?;
        modified <= date
    } else {
        true
    };
    let if_match = if headers.contains_key("If-Match") {
        let matches: bool = headers
            .get_all("If-Match")
            .map(|e| e.to_str().unwrap().trim_matches('"'))
            .any(|e| e == &etag);
        matches
    } else {
        true
    };
    if !unmodified_since || !if_match {
        return Ok(precondition_failed!());
    }

    match delete(&conn, Id(file.id)).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            err!("{}", e);
            Ok(internal_server_error!())
        }
    }
}
