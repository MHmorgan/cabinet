use crate::get_db_conn;
use actix_web::{HttpResponse, Result};
use mhlog::err;
use std::collections::HashMap;

#[actix_web::get("/status")]
pub async fn get() -> Result<HttpResponse> {
    use crate::database;
    let mut stats = HashMap::new();
    let conn = get_db_conn();

    let files: usize = match database::file::count(&conn).await {
        Ok(n) => n,
        Err(e) => {
            err!("Failed to get files count: {}", e);
            return Ok(internal_server_error!());
        }
    };
    stats.insert("files", files);

    let dirs: usize = match database::dir::count(&conn).await {
        Ok(n) => n,
        Err(e) => {
            err!("Failed to get directories count: {}", e);
            return Ok(internal_server_error!());
        }
    };
    stats.insert("directories", dirs);

    let bps: usize = match database::boilerplate::count(&conn).await {
        Ok(n) => n,
        Err(e) => {
            err!("Failed to get boilerplates count: {}", e);
            return Ok(internal_server_error!());
        }
    };
    stats.insert("boilerplates", bps);

    return Ok(HttpResponse::Ok().json(&stats));
}
