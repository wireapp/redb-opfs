use std::io::Result;

pub(crate) trait FileLen {
    fn len(&self) -> Result<u64>;
}

#[cfg(not(target_family = "wasm"))]
impl FileLen for std::fs::File {
    fn len(&self) -> Result<u64> {
        self.metadata().map(|metadata| metadata.len())
    }
}

#[cfg(target_family = "wasm")]
impl FileLen for crate::file::File {
    fn len(&self) -> Result<u64> {
        self.size()
    }
}
