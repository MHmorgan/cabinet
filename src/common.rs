
#[macro_export]
macro_rules! return_error {
    ($($arg:tt)+) => {
        return Err($crate::CabinetError::Other(format!($($arg)+)))
    };
}

/*******************************************************************************
 *                                                                             *
 * HTTP response macros
 *                                                                             *
 *******************************************************************************/

//
// 304 Not Modified
//
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

//
// 400 Bad Request
//
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

//
// 404 Not Found
//
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

//
// 412 Precondition Failed
//
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

//
// 413 Paiload Too Large
//
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

//
// 500 Internal Server Error
//
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

/*******************************************************************************
 *                                                                             *
 * Query helpers
 *                                                                             *
 *******************************************************************************/

#[macro_export]
macro_rules! query_row {
    ($conn:ident, $query:tt => $func:expr) => {
        $conn
            .prepare($query)?
            .query_row(&[], $func)
    };
    ($conn:ident, $query:tt => $func:expr; $($arg:tt),*) => {
        $conn
            .prepare($query)?
            .query_row(params![$($arg)*], $func)
    };
}

#[macro_export]
macro_rules! query_exists {
    ($conn:ident, $query:tt) => {
        $conn
            .prepare($query)?
            .exists(&[])
    };
    ($conn:ident, $query:tt; $($arg:tt),*) => {
        $conn
            .prepare($query)?
            .exists(params![$($arg)*])
    };
}

#[macro_export]
macro_rules! execute {
    ($conn:ident, $query:tt) => {
        $conn
            .prepare($query)?
            .execute(&[])
    };
    ($conn:ident, $query:tt; $($arg:tt),*) => {
        $conn
            .prepare($query)?
            .execute(params![$($arg)*])
    };
}

