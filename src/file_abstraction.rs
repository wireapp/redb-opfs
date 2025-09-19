use std::io::Result;

pub(crate) trait FileAbstraction: Sized {
    /// Open the specified path.
    ///
    /// Must have the following conditions/flags set at initialization:
    ///
    /// - readable
    /// - writeable
    /// - created if does not exist
    /// - _not_ truncated
    /// - initial cursor position at 0
    async fn open(path: &str) -> Result<Self>;

    /// Get the length of this file in bytes.
    fn len(&self) -> Result<u64>;
}

#[cfg(not(target_family = "wasm"))]
impl FileAbstraction for std::fs::File {
    async fn open(path: &str) -> Result<Self> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
    }

    fn len(&self) -> Result<u64> {
        self.metadata().map(|metadata| metadata.len())
    }
}

#[cfg(target_family = "wasm")]
impl FileAbstraction for crate::file::File {
    async fn open(path: &str) -> Result<Self> {
        <Self>::open(path).await.map_err(crate::Error::into_inner)
    }

    fn len(&self) -> Result<u64> {
        self.size()
    }
}
