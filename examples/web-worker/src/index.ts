import { Semaphore, type Lock } from "./semaphore";
import type { Message, Response } from "./types";
import { dlog } from "./dlog";

class OpfsUser {
    private dispatchSemaphore: Semaphore;
    private responseSemaphore: Semaphore;
    private responseLock: Lock | null;
    private response: Response | null;
    private worker: Worker;

    constructor(label?: string, workerPath: string = "./src/worker.ts") {
        this.dispatchSemaphore = new Semaphore(`${label ?? workerPath} dispatch`, 1);
        this.responseSemaphore = new Semaphore(`${label ?? workerPath} response`, 1);
        this.response = null;
        this.responseLock = null;

        this.worker = new Worker(workerPath, {
            preload: "../../ts/gen/redb-opfs"
        });
        this.worker.addEventListener("error", (event) => {
            throw new Error(event.message);
        });
        this.worker.addEventListener("message",
            (event) => {
                dlog("opfsu: received message from worker");
                this.response = event.data;
                this.releaseResponseLock();
            });
    }

    private releaseResponseLock() {
        if (this.responseLock !== null) {
            this.responseLock.release();
        }
    }

    private async dispatch(message: Message): Promise<Response> {
        const dispatchLock = await this.dispatchSemaphore.acquire();

        try {
            // get the response lock. This will be released when we receive a response.
            // This means we can efficiently block on a reacquisition attempt.
            this.responseLock = await this.responseSemaphore.acquire();
            dlog(`opfsu: dispatching message to worker`);

            this.worker.postMessage(message);
            // now wait until the response is received (and immediately release it)
            dlog(`opfsu: waiting for response from worker`);
            (await this.responseSemaphore.acquire()).release();
            // now we have a response we can return
            const response = this.response ?? new Uint8Array();
            this.response = null;
            return response;
        } finally {
            dispatchLock.release()
            this.releaseResponseLock()
        }
    }

    /** Store some data in Opfs */
    async store(offset: bigint, data: Uint8Array): Promise<void> {
        dlog(`opfsu: storing ${data.length} bytes at offset ${offset}`);
        await this.dispatch({ op: "store", offset, data })
    }

    /** Retrieve `size` bytes from Opfs */
    async load(offset: bigint, size: number): Promise<Uint8Array> {
        dlog(`opfsu: loading ${size} bytes at offset ${offset}`);
        return await this.dispatch({ op: "load", offset, size })
    }
}

// lets cache some squares in opfs so we never have to recompute them
const opfsUser = new OpfsUser("opfs");

const squares = new Uint8Array(16);
for (var i = 0; i < 16; i++) {
    squares[i] = i * i;
}

// store the squares
console.log("main: storing data")
await opfsUser.store(BigInt(0), squares);

// what's the square of 4?
console.log("main: retrieving data")
const foursquared = (await opfsUser.load(BigInt(4), 1))[0];
console.log("main: square of 4 is: ", foursquared);
