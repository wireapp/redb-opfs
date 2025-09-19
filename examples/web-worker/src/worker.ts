declare var self: Worker;

import init, { OpfsBackend } from "../../../ts/gen/redb-opfs"
import { dlog } from "./dlog";
import { isMessage, type Response } from "./types";

dlog("worker: initializing wasm");
await init();
dlog("worker: initializing OpfsBackend")
const backend = await OpfsBackend.open("my-db");


dlog("worker: registering message handler");
self.addEventListener("message",
    // note this is _not_ async! The magic here is using OPFS synchronously.
    (event) => {
        dlog("worker: processing message");

        var ret: Response = new Uint8Array();
        try {
            if (!isMessage(event.data)) {
                console.error("could not decipher event.data as message");
                return;
            }

            dlog(`worker: processing ${event.data.op} (${event.data.id})`);

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

            dlog(`worker: posting response to ${event.data.id} (${ret.length} bytes)`)
            self.postMessage({ id: event.data.id, data: ret });
        } catch (e) {
            dlog(`worker: responding with error to ${event.data.id} (${e})`)
            self.postMessage({ id: event.data.id, error: String(e) })
        }
    });

// send the handshake ready message so the `OpfsUser` knows we can now receive messages
self.postMessage({ type: "ready" });
