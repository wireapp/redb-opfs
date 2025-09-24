use std::cell::OnceCell;

use redb::{Database, ReadTransaction, ReadableDatabase as _, TableDefinition, WriteTransaction};
use redb_opfs::OpfsBackend;
use wasm_bindgen::JsValue;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

thread_local! {
    static DATABASE: OnceCell<Database> = OnceCell::new();
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Duration since the Unix epoch in milliseconds
pub type Timestamp = u32;

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

    let tx = write_tx()?;
    {
        // create the tables
        let _ = tx.open_table(CLICK_TABLE)?;
    }
    tx.commit()?;

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
///
/// The `timestamp` parameter is the number of milliseconds after the unix epoch.
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn click(timestamp: u32) -> Result<()> {
    let mut tx = write_tx()?;
    tx.set_durability(redb::Durability::Immediate)
        .map_err(Error::Durability)?;
    {
        let mut table = tx.open_table(CLICK_TABLE)?;
        table.insert(timestamp, ())?;
    }
    tx.commit()?;

    Ok(())
}

/// Clicks since a timestamp.
///
/// The timestamp, if present, is expressed as the number of milliseconds after the unix epoch.
///
/// If `millis_after_epoch` is `None`, returns the total number of clicks.
#[cfg_attr(target_family = "wasm", wasm_bindgen(js_name = clicksSince))]
pub fn clicks_since(millis_after_epoch: Option<u32>) -> Result<u32> {
    let tx = read_tx()?;
    let table = tx.open_table(CLICK_TABLE)?;
    let n_clicks = match millis_after_epoch {
        None => table.range::<Timestamp>(..)?.count() as _,
        Some(millis_after_epoch) => {
            let lower_bound = millis_after_epoch as Timestamp;
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
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("db: {0}")]
    Db(#[from] redb::DatabaseError),
    #[error("table: {0}")]
    Table(#[from] redb::TableError),
    #[error("storage: {0}")]
    Storage(#[from] redb::StorageError),
    #[error("commit: {0}")]
    Commit(#[from] redb::CommitError),
    #[error("transaction: {0}")]
    Transaction(#[from] redb::TransactionError),
    #[error("durability: {0}")]
    Durability(redb::SetDurabilityError),
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
