import type { Message, Response } from "./types";
import { dlog } from "./dlog";

type PendingRequest = {
    resolve: (value: Response) => void;
    reject: (reason?: any) => void;
}

export class OpfsUser {
    private worker: Worker;
    private ready: Promise<void>;
    private pending = new Map<number, PendingRequest>();
    private nextId = 0;

    constructor(workerPath: string = "./worker.js") {
        this.worker = new Worker(workerPath, { type: "module" });
        this.worker.addEventListener("error", (e) => this._onError(e));

        // this promise will resolve exactly once when the worker sends the ready message,
        // then remain resolved forever, minimizing latency
        this.ready = new Promise(resolve => {
            const onReady = (event: MessageEvent) => {
                if (event.data?.type === "ready") {
                    this.worker.removeEventListener("message", onReady);
                    this.worker.addEventListener("message", (e) => this._onMessage(e));
                    resolve();
                }
            }
            this.worker.addEventListener("message", onReady);
        });
    }

    _onError(event: ErrorEvent) {
        console.error("worker error event:", event);
        throw new Error(event.message);
    }

    _onMessage(event: MessageEvent) {
        const { id, data, error } = event.data;
        dlog(`opfsu: received message from worker (${id})`);
        const pending = this.pending.get(id);
        if (!pending) return;

        this.pending.delete(id);
        if (error) pending.reject(error);
        else pending.resolve(data);
    }

    destroy() {
        this.worker.removeEventListener('error', this._onError);
        this.worker.removeEventListener('message', this._onMessage);
        this.worker.terminate()
    }

    // the dispatch function picks its own id
    private async dispatch(message: Omit<Message, "id">): Promise<Response> {
        // wait for worker to report that it is ready
        await this.ready;

        const id = this.nextId++;
        return new Promise<Response>((resolve, reject) => {
            dlog(`opfsu: dispatching (${id})`);
            this.pending.set(id, { resolve, reject });
            this.worker.postMessage({ id, ...message });
        })
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
