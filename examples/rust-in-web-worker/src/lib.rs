use std::{
    cell::OnceCell,
    time::{SystemTime, UNIX_EPOCH},
};

use redb::{Database, TableDefinition, WriteTransaction};
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
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub async fn init(db_name: &str) -> Result<()> {
    let backend = OpfsBackend::new(db_name).await?;
    let database = Database::builder().create_with_backend(backend)?;

    DATABASE.with(|database_cell| {
        database_cell
            .set(database)
            .map_err(|_| Error::AlreadyInitialized)
    })?;

    Ok(())
}

/// Record a click
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn click() -> Result<()> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();

    let tx = DATABASE.with(|database_cell| -> Result<WriteTransaction> {
        database_cell
            .get()
            .ok_or(Error::NotInitialized)?
            .begin_write()
            .map_err(Into::into)
    })?;
    {
        let mut table = tx.open_table(CLICK_TABLE)?;
        table.insert(timestamp, ())?;
    }
    tx.commit()?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database already initialized; do not re-init")]
    AlreadyInitialized,
    #[error("database not yet initialized; call `init`")]
    NotInitialized,
    #[error(transparent)]
    Opfs(#[from] redb_opfs::Error),
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

impl From<Error> for JsValue {
    fn from(value: Error) -> JsValue {
        js_sys::Error::new(&value.to_string()).into()
    }
}
