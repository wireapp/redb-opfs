//! [`OpfsBackend`] mplements a [`StorageBackend`] which delegates to [OPFS].
//!
//! For obvious reasons this crate only works at all when compiled for WASM. For other compilation targets,
//! manually instantiate a different filesystem.
//!
//! [OPFS]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system

#[cfg(not(target_family = "wasm"))]
compile_error!("redb-opfs is only meaningful when targeting wasm");

use core::fmt;
use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use futures_lite::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _};
use redb::StorageBackend;
use wasm_bindgen_futures::spawn_local;
use wasm_sync::Condvar;
use web_fs::{File, OpenOptions};

/// Implementataion of a [`StorageBackend`] which delegates to [OPFS].
///
/// [OPFS]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system
pub struct OpfsBackend {
    path: PathBuf,
}

impl fmt::Debug for OpfsBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpfsBackend")
            .field("file", &"<opaque handle>")
            .finish()
    }
}

impl OpfsBackend {
    pub async fn new(path: impl AsRef<Path>) -> Self {
        // let file = OpenOptions::new()
        //     .read(true)
        //     .write(true)
        //     .create(true)
        //     .open(path)
        //     .await?;
        // Ok(Self { file })
        let path = path.as_ref().to_owned();
        Self { path }
    }

    /// Open the file at this path with conventional options
    ///
    /// - can read
    /// - can write
    /// - can create
    /// - no truncation
    /// - no append
    async fn open(path: impl AsRef<Path>) -> std::io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .await
    }

    // see https://docs.rs/wasm_sync/latest/wasm_sync/struct.Condvar.html#method.wait for the pattern used here
    fn execute_async<T, Op, Fut>(&self, operation: Op) -> T
    where
        T: 'static,
        // FIXME: it's kind of terrible for performance that we only store the path and need
        // to reopen the file for every single operation, but that's a problem for Future Me
        Op: 'static + FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = T>,
    {
        let pair = Arc::new((Mutex::new(None), Condvar::new()));

        {
            let pair = pair.clone();
            let path = self.path.clone();
            spawn_local(async move {
                let (lock, cvar) = &*pair;
                let t = Some(operation(path).await);
                // note the sequencing here: we do not hold the guard across an .await
                let mut guard = lock.lock().expect("non-poisoned lock");
                *guard = t;
                cvar.notify_one();
            });
        }

        {
            let (lock, cvar) = &*pair;
            let mut guard = lock.lock().expect("non-poisoned lock");
            while guard.is_none() {
                guard = cvar.wait(guard).expect("non-poisoned lock");
            }
        }

        let (lock, _cvar) = Arc::into_inner(pair).expect("other cvar in promise is dropped by now");
        lock.into_inner()
            .expect("non-poisoned lock")
            .expect("if guard were not Some we should not have escaped the guard loop")
    }
}

impl StorageBackend for OpfsBackend {
    fn len(&self) -> Result<u64, std::io::Error> {
        self.execute_async(async move |path| {
            let len = Self::open(path).await?.metadata().await?.len();
            Ok(len)
        })
    }

    fn read(&self, offset: u64, out: &mut [u8]) -> Result<(), std::io::Error> {
        let out_len = out.len();
        self.execute_async(async move |path| {
            let mut file = Self::open(path).await?;
            file.seek(SeekFrom::Start(offset)).await?;
            // FIXME: allocating our own buffer here is another performance hit
            let mut buf = vec![0; out_len];
            file.read_exact(&mut buf).await?;
            Ok(buf)
        })
        .map(|data| out.copy_from_slice(&data))
    }

    fn set_len(&self, len: u64) -> Result<(), std::io::Error> {
        self.execute_async(async move |path| {
            let mut file = Self::open(path).await?;
            file.set_len(len).await?;
            Ok(())
        })
    }

    fn sync_data(&self) -> Result<(), std::io::Error> {
        // FIXME: once we have a persistent file handle, this should delegate to [`File::sync_data`],
        // but for now as we reopen / save the file every time anyway, we can get away with not doing
        // anything
        Ok(())
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<(), std::io::Error> {
        let data = data.to_owned();
        self.execute_async(async move |path| {
            let mut file = Self::open(path).await?;
            file.seek(SeekFrom::Start(offset)).await?;
            file.write_all(&data).await?;
            Ok(())
        })
    }
}
