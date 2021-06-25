use std::path::PathBuf;
use std::sync::Mutex;
// use async_std::path::PathBuf;

#[derive(Debug, Default, Clone)]
struct Context {
    /// The directory root for all data available for the server.
    root: PathBuf,
}

lazy_static! {
    static ref CTX: Mutex<Context> = {
        let ctx = Context {
            ..Default::default()
        };
        Mutex::new(ctx)
    };
}

pub fn init(root: PathBuf) {
    let mut ctx = CTX.lock().unwrap();
    ctx.root = root;
}

pub fn files_dir() -> PathBuf {
    let ctx = CTX.lock().unwrap();
    ctx.root.join("files")
}

pub fn boilerplate_dir() -> PathBuf {
    let ctx = CTX.lock().unwrap();
    ctx.root.join("boilerplates")
}

// -----------------------------------------------------------------------------
// HTTP response macros

#[macro_export]
macro_rules! not_modified {
    () => {
        actix_web::HttpResponse::NotModified()
            .body("304 Not Modified")
    };
    ($resp:ident) => {
        $resp.status(actix_web::http::StatusCode::NOT_MODIFIED)
            .body("304 Not Modified")
    };
    ($($arg:tt)+) => {
        actix_web::HttpResponse::NotModified()
            .body(format!("304 Not Modified: {}", format_args!($($arg)+)))
    };
}

#[macro_export]
macro_rules! bad_request {
    () => {
        actix_web::HttpResponse::BadRequest()
            .body("400 Bad Request")
    };
    ($($arg:tt)+) => {
        actix_web::HttpResponse::BadRequest()
            .body(format!("400 Bad Request: {}", format_args!($($arg)+)))
    };
}

#[macro_export]
macro_rules! not_found {
    () => {
        actix_web::HttpResponse::NotFound()
            .body("404 Not Found")
    };
    ($($arg:tt)+) => {
        actix_web::HttpResponse::NotFound()
            .body(format!("404 Not Found: {}", format_args!($($arg)+)))
    };
}

#[macro_export]
macro_rules! precondition_failed {
    () => {
        actix_web::HttpResponse::PreconditionFailed()
            .body("412 Precondition Failed")
    };
    ($($arg:tt)+) => {
        actix_web::HttpResponse::PreconditionFailed()
            .body(format!("412 Precondition Failed: {}", format_args!($($arg)+)))
    };
}

#[macro_export]
macro_rules! payload_too_large {
    () => {
        actix_web::HttpResponse::PreconditionFailed()
            .body("413 Payload Too Large")
    };
    ($($arg:tt)+) => {
        actix_web::HttpResponse::PreconditionFailed()
            .body(format!("413 Payload Too Large: {}", format_args!($($arg)+)))
    };
}

#[macro_export]
macro_rules! internal_server_error {
    () => {
        actix_web::HttpResponse::InternalServerError()
            .body("500 Internal Server Error")
    };
    ($($arg:tt)+) => {
        actix_web::HttpResponse::PreconditionFailed()
            .body(format!("500 Internal Server Error: {}", format_args!($($arg)+)))
    };
}
