use std::{
    cell::OnceCell,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use redb::{Database, ReadTransaction, ReadableDatabase as _, TableDefinition, WriteTransaction};
use redb_opfs::OpfsBackend;
use wasm_bindgen::JsValue;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

thread_local! {
    static DATABASE: OnceCell<Database> = OnceCell::new();
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Duration since the Unix epoch in microseconds
pub type Timestamp = u128;

const CLICK_TABLE: TableDefinition<Timestamp, ()> = TableDefinition::new("clicks");

/// Initialize the database
#[cfg_attr(target_family = "wasm", wasm_bindgen(js_name = initDb))]
pub async fn init_db(db_name: &str) -> Result<()> {
    let backend = OpfsBackend::new(db_name).await?;
    let database = Database::builder().create_with_backend(backend)?;

    DATABASE.with(|database_cell| {
        database_cell
            .set(database)
            .map_err(|_| Error::AlreadyInitialized)
    })?;

    Ok(())
}

fn write_tx() -> Result<WriteTransaction> {
    DATABASE.with(|database_cell| {
        database_cell
            .get()
            .ok_or(Error::NotInitialized)?
            .begin_write()
            .map_err(Into::into)
    })
}

fn read_tx() -> Result<ReadTransaction> {
    DATABASE.with(|database_cell| {
        database_cell
            .get()
            .ok_or(Error::NotInitialized)?
            .begin_read()
            .map_err(Into::into)
    })
}

/// Record a click
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn click() -> Result<()> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();

    let tx = write_tx()?;
    {
        let mut table = tx.open_table(CLICK_TABLE)?;
        table.insert(timestamp, ())?;
    }
    tx.commit()?;

    Ok(())
}

/// Clicks in last N seconds
///
/// If `n_seconds` is `None`, returns the total number of clicks.
#[cfg_attr(target_family = "wasm", wasm_bindgen(js_name = clicksInLastSeconds))]
pub fn clicks_in_last_seconds(n_seconds: Option<u32>) -> Result<u32> {
    let tx = read_tx()?;
    let table = tx.open_table(CLICK_TABLE)?;
    let n_clicks = match n_seconds {
        None => table.range::<Timestamp>(..)?.count() as _,
        Some(n_seconds) => {
            let lower_bound = (SystemTime::now() - Duration::from_secs(n_seconds as _))
                .duration_since(UNIX_EPOCH)?
                .as_micros();
            table.range(lower_bound..)?.count() as _
        }
    };

    Ok(n_clicks)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database already initialized; do not re-init")]
    AlreadyInitialized,
    #[error("database not yet initialized; call `init_db`")]
    NotInitialized,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Db(#[from] redb::DatabaseError),
    #[error(transparent)]
    Table(#[from] redb::TableError),
    #[error(transparent)]
    Storage(#[from] redb::StorageError),
    #[error(transparent)]
    Commit(#[from] redb::CommitError),
    #[error(transparent)]
    Transaction(#[from] redb::TransactionError),
    #[error("system time appears to be before unix epoch")]
    SystemTime(#[from] std::time::SystemTimeError),
}

#[cfg(target_family = "wasm")]
impl From<redb_opfs::Error> for Error {
    fn from(value: redb_opfs::Error) -> Error {
        value.into_inner().into()
    }
}

impl From<Error> for JsValue {
    fn from(value: Error) -> JsValue {
        js_sys::Error::new(&value.to_string()).into()
    }
}
