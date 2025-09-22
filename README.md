# `redb-opfs`: Implements a `StorageBackend` which delegates to OPFS

This allows deployment on `wasm32-unknown-unknown`.

> [!WARNING] The contents of this README are a statement of intent, not an accurate reflection of the current state of
> the project.
>
> Carefully inspect the code and/or generated documentation before relying on this library.

## Usage

It is important to understand that Redb's [`StorageBackend`
interface](https://docs.rs/redb/latest/redb/trait.StorageBackend.html) is fundamentally synchronous, and OPFS is
fundamentally asynchronous. There's a simple way to tie these two things
together--[`block_on`](https://docs.rs/futures-lite/latest/futures_lite/future/fn.block_on.html)--but that method is
illegal on the main thread, in order not to block the UI.

> [!IMPORTANT] The `OpfsBackend` instance **must** run on a web worker.

This gives rise to two use cases.

### Your Rust code is already running in a web worker

This case is nice and simple; everything stays within Rust. Just explicitly choose this backend when initializing your
`Database`:

```rust
use redb_opfs::OpfsBackend;

let database = redb::Builder::new()
  .create_with_backend(OpfsBackend::new("my-db")?)?;
```

### Your Rust code is running in the main thread

> [!NOTE] Running in this configuration introduces unavoidable performance penalties; when possible, you should prefer
> to run all your Rust code within a web worker to avoid these.

In this case we need to instantiate the `OpfsBackend` on a web worker and then instantiate the handle on the main
thread.

You'll want to use the `worker-shim.js` worker file to initialize the worker, and then hand that worker to the
`OpfsBackendHandle`

```js
import { WorkerHandle } from "./redb-opfs";

const redbOpfsWorker = new Worker("worker-shim.js");
const workerHandle = WorkerHandle(redbOpfsWorker);

// now pass that handle to your rust code, using a mechanism of your choice.
```

As you're writing your own Rust anyway, you have your own means of getting the handle into your code from there. To keep
life simple, there exists `impl TryFrom<JsValue> for WorkerHandle`.

Once you have that, usage is fairly simple:

```rust
use redb_opfs::WorkerHandle;

let worker_handle = WorkerHandle::try_from(my_js_value)?;
let database = redb::Builder::new()
  .create_with_backend(worker_hanndle)?;
```

## Building

### Prerequisites for WASM

- [`wasm-bindgen` cli](https://github.com/wasm-bindgen/wasm-bindgen?tab=readme-ov-file#install-wasm-bindgen-cli)
- [wasm-pack](https://github.com/drager/wasm-pack)
- [GNU Make](https://www.gnu.org/software/make/)

## Examples

### Web Worker

This example demonstrates the simplest possible case: `redb-opfs` is compiled to WASM and deployed, where it is used
only from Typescript code without interacting at all with other Rust code. In this use case, the only thing it gives us
is a fully-synchronous interface to OPFS.

#### Prerequisites

- [bun](https://bun.com/)
- [miniserve](https://github.com/svenstaro/miniserve)

#### Usage

- `make web-worker-example`
- point your browser at any of the listed URLs the server has bound itself to

## License

Licensed only under [GPL-3.0](./LICENSE).

### Contribution

Any contribution intentionally submitted for inclusion in this work shall be licensed as above, without any additional
terms or conditions.
