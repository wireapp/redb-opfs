//! [`OpfsBackend`] mplements a [`StorageBackend`] which delegates to [OPFS] when built for wasm.
//!
//! [OPFS]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system

#[cfg(target_family = "wasm")]
mod error;
#[cfg(not(target_family = "wasm"))]
mod file {
    pub use std::fs::File;
    use std::fs::OpenOptions;

    pub async fn open_writeable(path: &str) -> std::io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
    }
}
#[cfg(target_family = "wasm")]
mod file;
mod file_len;

use std::io::{Read as _, Seek as _, SeekFrom, Write as _};

use file::{File, open_writeable};
use file_len::FileLen as _;
use parking_lot::Mutex;
use redb::StorageBackend;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
pub use error::Error;

#[cfg(not(target_family = "wasm"))]
type Error = std::io::Error;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;
type IoResult<T> = std::io::Result<T>;

/// Implementataion of a [`StorageBackend`] which delegates to [OPFS] when built for wasm.
///
/// **IMPORTANT**: This can only ever be used within a web worker.
/// This _may_ instantiate within the main thread, but as it blocks internally,
/// it will fail at runtime on the main thread if you attempt to actually use it.
///
/// In native contexts, this targets the local file system.
///
/// [OPFS]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
#[derive(Debug)]
pub struct OpfsBackend {
    file: Mutex<File>,
}

// Safety: when targeting wasm, we're really working in a single-threaded context anyway, so
// literally everything is trivially `Send`, because there are no other threads to send it to.
//
// Note that we only need to manually implement this for wasm; in native contexts, `async_fs::File`
// already implements `Send`.
#[cfg(target_family = "wasm")]
unsafe impl Send for OpfsBackend {}

// Safety: when targeting wasm, we don't have multiple threads to send things between, but we
// very often need to coordinate between various async contexts. For this reason we put a mutex
// around the file handle, so contention is explicitly resolved.
#[cfg(target_family = "wasm")]
unsafe impl Sync for OpfsBackend {}

#[cfg_attr(target_family = "wasm", wasm_bindgen)]
impl OpfsBackend {
    /// Open the file at the specified path.
    #[cfg_attr(target_family = "wasm", wasm_bindgen(js_name = open))]
    pub async fn new(path: &str) -> Result<Self> {
        let file = open_writeable(path).await?;
        let file = Mutex::new(file);
        Ok(Self { file })
    }
}

impl StorageBackend for OpfsBackend {
    fn len(&self) -> IoResult<u64> {
        self.file.lock().len()
    }

    fn set_len(&self, len: u64) -> IoResult<()> {
        self.file.lock().set_len(len)
    }

    fn sync_data(&self) -> IoResult<()> {
        self.file.lock().flush()
    }

    fn read(&self, offset: u64, out: &mut [u8]) -> IoResult<()> {
        let mut guard = self.file.lock();
        guard.seek(SeekFrom::Start(offset))?;
        guard.read_exact(out)?;
        Ok(())
    }

    fn write(&self, offset: u64, data: &[u8]) -> IoResult<()> {
        let mut guard = self.file.lock();
        guard.seek(SeekFrom::Start(offset))?;
        guard.write_all(data)?;
        Ok(())
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
#[expect(clippy::len_without_is_empty)]
impl OpfsBackend {
    /// Returns the size of the file, in bytes
    //
    // Files have length but no trivial `is_empty` impl, so we skip that
    pub fn len(&self) -> Result<u64> {
        <Self as StorageBackend>::len(self).map_err(Into::into)
    }

    /// Reads some bytes from the file at the given offset.
    pub fn read(&self, offset: u64, out: &mut [u8]) -> Result<()> {
        <Self as StorageBackend>::read(self, offset, out).map_err(Into::into)
    }

    /// Truncates or extends the underlying file, updating the size of this file to become `size`.
    ///
    /// If `size` is less than the current file's size, then the file will be shrunk.
    /// If it is greater than the current file's size, then the file will be extended to `size`
    /// and have all intermediate data filled with 0s.
    ///
    /// The file's cursor is not changed. In particular, if the cursor was at the end of the file
    /// and the file is shrunk with this operaiton, the cursor will now be past the end.
    pub fn set_len(&self, len: u64) -> Result<()> {
        <Self as StorageBackend>::set_len(self, len).map_err(Into::into)
    }

    /// Attempts to sync all OS-internal file content to disk. This might not synchronize file metadata.
    pub fn sync_data(&self) -> Result<()> {
        <Self as StorageBackend>::sync_data(self).map_err(Into::into)
    }

    /// Writes some bytes to the file at the given offset.
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<()> {
        <Self as StorageBackend>::write(self, offset, data).map_err(Into::into)
    }
}
