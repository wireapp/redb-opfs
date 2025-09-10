# `redb-opfs`: Implements a `StorageBackend` which delegates to OPFS

This allows deployment on `wasm32-unknown-unknown`.

> [!WARNING]
> The contents of this README are a statement of intent, not an accurate reflection of the current state of the project.
>
> Carefully inspect the code and/or generated documentation before relying on this library.

## Usage

- Add this dependency to your project:

  ```sh
  cargo add redb-opfs
  ```

- Explicitly choose this backend when initializing your `Database`:

  ```rust
  use redb_opfs::OpfsBackend;

  let database = redb::Builder::new()
    .create_with_backend(OpfsBackend::new("my-db"))?;
  ```

- Go nuts!

## Building

### Prerequisites for WASM

- [`wasm-bindgen` cli](https://github.com/wasm-bindgen/wasm-bindgen?tab=readme-ov-file#install-wasm-bindgen-cli)
- [wasm-pack](https://github.com/drager/wasm-pack)
- [GNU Make](https://www.gnu.org/software/make/)


## License

Licensed only under [GPL-3.0](./LICENSE).

### Contribution

Any contribution intentionally submitted for inclusion in this work shall be licensed as above, without any additional terms or conditions.
