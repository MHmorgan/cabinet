//! The database module provides the interface for interacting
//! with the database for the rest of the server.
//! 
//! All conversion to and from rust types are handled by this module.

use rusqlite::{Connection, Result as RResult};

pub mod file;
pub mod dir;
pub mod boilerplate;

/// Module-internal interface
///
/// Required functionality:
///  1. Execute a single statement
///  2. Query a single row
///  3. Query multiple rows

pub async fn create_tables(conn: &Connection) -> RResult<()> {
    let sql = include_str!("tables.sql");
    conn.execute_batch(sql)
}
