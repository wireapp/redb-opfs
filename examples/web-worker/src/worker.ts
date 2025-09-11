declare var self: Worker;

import init, { OpfsBackend } from "../../../ts/gen/redb-opfs"
import { dlog } from "./dlog";
import { isMessage, type Response } from "./types";

// manually load the module.
// this may not be strictly necessary; we get the same error
// if we just omit all parameters to `init`
const WASM_PATH = "../../../ts/gen/redb-opfs_bg.wasm";
const wasm_url = new URL(WASM_PATH, import.meta.url)
const wasm_file = Bun.file(wasm_url);
const wasm_buffer = await wasm_file.arrayBuffer();

const module = new WebAssembly.Module(wasm_buffer);

dlog("worker: initializing wasm");
await init({ module_or_path: module });
dlog("worker: initializing OpfsBackend")
const backend = await OpfsBackend.open("my-db");


dlog("worker: registering message handler");
self.addEventListener("message",
    (event) => {
        dlog("worker: processing message");

        var ret: Response = new Uint8Array();
        try {
            if (!isMessage(event.data)) {
                console.error("could not decipher event.data as message");
                return;
            }

            dlog(`worker: processing ${event.data.op}`);

            switch (event.data.op) {
                case "store":
                    const writeData = event.data.data ?? new Uint8Array();
                    backend.write(event.data.offset, writeData);
                    backend.sync_data()
                    break;
                case "load":
                    ret = new Uint8Array(event.data.size ?? 0);
                    backend.sync_data()
                    backend.read(event.data.offset, ret);
                    break;
            }
        } finally {
            // always return something
            dlog(`worker: posting response (${ret.length} bytes)`)
            self.postMessage(ret)
        }
    });
