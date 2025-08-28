//! [`OpfsBackend`] mplements a [`StorageBackend`] which delegates to [OPFS] when built for wasm.
//!
//! [OPFS]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system

use std::{io::SeekFrom, path::Path};

use async_lock::Mutex;
use futures_lite::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _, future::block_on};
use redb::StorageBackend;
use tokio_fs_ext::{File, OpenOptions};

type Result<T> = std::io::Result<T>;

/// Implementataion of a [`StorageBackend`] which delegates to [OPFS] when built for wasm.
///
/// In native contexts, this targets the local file system.
///
/// [OPFS]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system
#[derive(Debug)]
pub struct OpfsBackend {
    file: Mutex<File>,
}

// Safety: when targeting wasm, we're really working in a single-threaded context anyway, so
// literally everything is trivially `Send`, because there are no other threads to send it to.
//
// Note that we only need to manually implement this for wasm; in native contexts, `tokio_fs_ext`
// is already built to
#[cfg(target_family = "wasm")]
unsafe impl Send for OpfsBackend {}

// Safety: when targeting wasm, we don't have multiple threads to send things between, but we
// very often need to coordinate between various async contexts. For this reason we put a mutex
// around the file handle, so contention is explicitly resolved.
#[cfg(target_family = "wasm")]
unsafe impl Sync for OpfsBackend {}

impl OpfsBackend {
    /// Open the file at the specified path.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .await?;
        let file = Mutex::new(file);
        Ok(Self { file })
    }
}

impl StorageBackend for OpfsBackend {
    fn len(&self) -> Result<u64> {
        block_on(async {
            self.file
                .lock()
                .await
                .metadata()
                .await
                .map(|metadata| metadata.len())
        })
    }

    fn read(&self, offset: u64, out: &mut [u8]) -> Result<()> {
        block_on(async {
            let mut guard = self.file.lock().await;
            guard.seek(SeekFrom::Start(offset)).await?;
            guard.read_exact(out).await?;
            Ok(())
        })
    }

    fn set_len(&self, len: u64) -> Result<()> {
        block_on(async { self.file.lock().await.set_len(len).await })
    }

    fn sync_data(&self) -> Result<()> {
        block_on(async { self.file.lock().await.sync_data().await })
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<()> {
        block_on(async {
            let mut guard = self.file.lock().await;
            guard.seek(SeekFrom::Start(offset)).await?;
            guard.write_all(data).await?;
            Ok(())
        })
    }
}
