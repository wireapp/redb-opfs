use std::{
    io::{self, ErrorKind, Read, Seek, Write},
    path::{Component, Path, PathBuf},
};

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DedicatedWorkerGlobalScope, FileSystemDirectoryHandle, FileSystemFileHandle,
    FileSystemGetDirectoryOptions, FileSystemGetFileOptions, FileSystemReadWriteOptions,
    FileSystemSyncAccessHandle,
};

use super::{Error, Result};

pub async fn open_writeable(path: impl AsRef<Path>) -> Result<File> {
    File::open(path).await
}

/// A blocking File abstraction that operates on OPFS via a [`FileSystemSyncAccessHandle`].
///
/// Because this is blocking, it can only run in the context of a web worker, i.e. a [`DedicatedWorkerGlobalScope`].
#[derive(Debug)]
pub(crate) struct File {
    pub(crate) handle: FileSystemSyncAccessHandle,
    pos: u64,
}

impl File {
    pub async fn open(path: impl AsRef<Path>) -> Result<File> {
        let path = virtualize_path(path)?;
        let name = path
            .file_name()
            .ok_or(io::Error::from(ErrorKind::InvalidFilename))?
            .to_string_lossy();

        // in a perfect world, it would be
        //   let parent_handle = path.parent().map(open_dir).unwrap_or_else(root).await?;
        // but we can't do that as each `impl Future` is a different type, even if the
        // outputs resolve to the same type.
        let parent_handle = match path.parent() {
            Some(parent) => open_dir(parent).await?,
            None => root().await?,
        };

        let file_handle = get_file_handle(&name, &parent_handle).await?;

        Ok(File {
            handle: file_handle,
            pos: 0,
        })
    }

    pub fn size(&self) -> io::Result<u64> {
        self.handle
            .get_size()
            .map(|size| size as _)
            .map_err(Error::to_io)
    }

    /// Truncates or extends the underlying file, updating the size of this file to become `size`.
    ///
    /// If `size` is less than the current file's size, then the file will be shrunk. If it is greater
    /// than the currrent file's size, then the file will be extended to `size` and have all intermediate
    /// data filled with 0s.
    ///
    /// The file's cursor is not changed. In particular, if the cursor was at the end of the file and
    /// the file was shrunk using this operation, the cursor will now be past the end.
    ///
    /// If the requested length is greater than 9007199254740991 (max safe integer in a floating-point context),
    /// this will produce an error.
    pub fn set_len(&mut self, size: u64) -> io::Result<()> {
        const MAX_SAFE_INT: u64 = js_sys::Number::MAX_SAFE_INTEGER as _;
        if size > MAX_SAFE_INT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("requested size {size} too large, max allowed is {MAX_SAFE_INT}"),
            ));
        }
        self.handle
            .truncate_with_f64(size as _)
            .map_err(Error::to_io)
    }

    /// Flush any pending changes to the file system.
    pub fn flush(&self) -> io::Result<()> {
        self.handle.flush().map_err(Error::to_io)
    }

    fn options(&self) -> FileSystemReadWriteOptions {
        let options = FileSystemReadWriteOptions::new();
        options.set_at(self.pos as _);
        options
    }
}

impl Seek for File {
    fn seek(&mut self, seek_from: io::SeekFrom) -> io::Result<u64> {
        // `SeekFrom` semantics: https://doc.rust-lang.org/nightly/std/io/enum.SeekFrom.html
        self.pos = match seek_from {
            io::SeekFrom::Start(offset) => offset,
            io::SeekFrom::End(offset) => {
                self.size()?.checked_add_signed(offset).ok_or_else(|| {
                    io::Error::new(
                        ErrorKind::InvalidInput,
                        "over/underflow seeking from file end",
                    )
                })?
            }
            io::SeekFrom::Current(offset) => {
                self.pos.checked_add_signed(offset).ok_or_else(|| {
                    io::Error::new(
                        ErrorKind::InvalidInput,
                        "over/underflow seeking from current position",
                    )
                })?
            }
        };
        Ok(self.pos)
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self
            .handle
            .read_with_u8_array_and_options(buf, &self.options())
            .map_err(Error::to_io)? as u64;
        self.pos += bytes_read;
        Ok(bytes_read as _)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let bytes_written = self
            .handle
            .write_with_u8_array_and_options(buf, &self.options())
            .map_err(Error::to_io)? as u64;
        self.pos += bytes_written;
        Ok(bytes_written as _)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.handle.flush().map_err(Error::to_io)
    }
}

/// Construct a normalized version of the input path
fn virtualize_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let mut out = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            std::path::Component::RootDir => out.clear(),
            std::path::Component::CurDir => {}
            std::path::Component::Normal(normal) => out.push(normal),
            std::path::Component::Prefix(_) | std::path::Component::ParentDir => {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "only normal path components are supported",
                )
                .into());
            }
        }
    }

    Ok(out)
}

async fn root() -> Result<FileSystemDirectoryHandle> {
    let storage = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()))
        .navigator()
        .storage();

    let root_handle = JsFuture::from(storage.get_directory())
        .await?
        .dyn_into::<FileSystemDirectoryHandle>()?;

    Ok(root_handle)
}

async fn open_dir(path: impl AsRef<Path>) -> Result<FileSystemDirectoryHandle> {
    async fn get_dir_handle(
        parent: &FileSystemDirectoryHandle,
        path: &str,
    ) -> Result<FileSystemDirectoryHandle> {
        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(true);

        JsFuture::from(parent.get_directory_handle_with_options(path, &options))
            .await?
            .dyn_into::<FileSystemDirectoryHandle>()
            .map_err(Into::into)
    }

    let mut handle = root().await?;
    for component in path.as_ref().components() {
        let Component::Normal(component) = component else {
            // shouldn't happen though because we always virtualize ahead of time
            return Err(Error::ad_hoc(format!(
                "non-normal component in path: {component:?}"
            )));
        };
        let component = component.to_string_lossy();
        handle = get_dir_handle(&handle, &component).await?;
    }

    Ok(handle)
}

async fn get_file_handle(
    name: &str,
    dir: &FileSystemDirectoryHandle,
) -> Result<FileSystemSyncAccessHandle> {
    let options = FileSystemGetFileOptions::new();
    options.set_create(true);
    let file_handle = JsFuture::from(dir.get_file_handle_with_options(name, &options))
        .await?
        .dyn_into::<FileSystemFileHandle>()?;

    let file_handle = JsValue::from(file_handle);
    let create_sync_access_handle_promise =
        Reflect::get(&file_handle, &"createSyncAccessHandle".into())?
            .dyn_into::<Function>()?
            .call0(&file_handle)?
            .dyn_into::<Promise>()?;
    let sync_access_handle = JsFuture::from(create_sync_access_handle_promise)
        .await?
        .dyn_into::<FileSystemSyncAccessHandle>()?;
    Ok(sync_access_handle)
}
