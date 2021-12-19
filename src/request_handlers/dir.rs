use crate::get_db_conn;
use actix_web::{web, HttpResponse, Result};
use mhlog::err;

#[actix_web::get("/dirs/{dir:.*}")]
pub async fn get(web::Path(dir_path): web::Path<String>) -> Result<HttpResponse> {
    use crate::database::dir::{content, DirIdentifier};
    use crate::CabinetError::NotFound;

    //
    // Get directory content
    //
    let conn = get_db_conn();
    let content = match content(&conn, DirIdentifier::Path(dir_path.as_ref())).await {
        Ok(content) => content,
        Err(NotFound) => return Ok(not_found!("{}", &dir_path)),
        Err(e) => {
            err!("Failed getting directory content: {}", e);
            return Ok(internal_server_error!());
        }
    };

    //
    // Create and return response object
    //
    let names: Vec<_> = content.iter().map(ToString::to_string).collect();
    let mut resp = HttpResponse::Ok();
    Ok(resp.json(&names))
}

#[actix_web::put("/dirs/{dir:.*}")]
pub async fn put(web::Path(dir_path): web::Path<String>) -> Result<HttpResponse> {
    use crate::database::dir::DirIdentifier::Path;
    use crate::database::dir::{create, exists};

    //
    // Check if the directory already exists
    //
    let conn = get_db_conn();
    let exists = match exists(&conn, Path(dir_path.as_ref())).await {
        Ok(exists) => exists,
        Err(e) => {
            err!("Failed checking if directory exists: {}", e);
            return Ok(internal_server_error!());
        }
    };
    if exists {
        return Ok(HttpResponse::NoContent().finish());
    }

    //
    // Create directory if it doesn't exist
    //
    match create(&conn, dir_path.as_ref()).await {
        Ok(_) => Ok(HttpResponse::Created().finish()),
        Err(e) => {
            err!("Failed creating directory: {}", e);
            Ok(internal_server_error!())
        }
    }
}

#[actix_web::delete("/dirs/{dir:.*}")]
pub async fn delete(web::Path(dir_path): web::Path<String>) -> Result<HttpResponse> {
    use crate::database::dir::DirIdentifier::{Id, Path};
    use crate::database::dir::{content, delete, fetch};
    use crate::CabinetError::NotFound;

    //
    // Fetch requested directory
    //
    let conn = get_db_conn();
    let dir_entry = match fetch(&conn, Path(dir_path.as_ref())).await {
        Ok(dir) => dir,
        Err(NotFound) => return Ok(not_found!("{}", &dir_path)),
        Err(e) => {
            err!("Failed fetching directory for deleting: {}", e);
            return Ok(internal_server_error!());
        }
    };

    //
    // Check that the directory is empty before deleting
    //
    let content = match content(&conn, Id(dir_entry.id)).await {
        Ok(content) => content,
        Err(e) => {
            err!("Failed to get content of directory: {}", e);
            return Ok(internal_server_error!());
        }
    };
    if content.len() > 0 {
        return Ok(bad_request!("directory not empty"));
    }

    match delete(&conn, Id(dir_entry.id)).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            err!("Failed to delete directory: {}", e);
            return Ok(internal_server_error!());
        }
    }
}
