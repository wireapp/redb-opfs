import type { InitResponse, WorkerRequest, WorkerRequestBody, WorkerResponse } from "./messages";

type PendingRequest = {
    resolve: (value: WorkerResponse) => void;
    reject: (reason?: any) => void;
}

export class ClickTracker {
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
            const onReady = (event: MessageEvent<InitResponse>) => {
                if (event.data?.type === "init") {
                    console.log("ct: received init message from worker");
                    this.worker.removeEventListener("message", onReady);
                    this.worker.addEventListener("message", (e) => this._onMessage(e));
                    resolve();
                } else {
                    console.error("received unxpected message from worker before init:", event.data);
                    throw new Error("received unexpected message from worker before init");
                }
            }
            this.worker.addEventListener("message", onReady);
        });
    }

    _onError(event: ErrorEvent) {
        console.error("worker error event:", event);
        throw new Error(event.message);
    }

    _onMessage(event: MessageEvent<WorkerResponse>) {
        const id = event.data.id;
        const pending = this.pending.get(id);
        if (!pending) return;

        this.pending.delete(id);
        pending.resolve(event.data);
    }


    destroy() {
        this.worker.removeEventListener('error', this._onError);
        this.worker.removeEventListener('message', this._onMessage);
        this.worker.terminate()
    }

    // the dispatch function picks its own id
    private async dispatch(body: WorkerRequestBody): Promise<WorkerResponse> {
        // wait for worker to report that it is ready
        await this.ready;

        const id = this.nextId++;
        const request: WorkerRequest = { id, body, };
        return new Promise<WorkerResponse>((resolve, reject) => {
            this.pending.set(id, { resolve, reject });
            this.worker.postMessage(request);
        })
    }

    /** Track a click */
    async click(): Promise<void> {
        await this.dispatch({ type: "click" });
    }

    /** Get the number of clicks in the last N seconds. If unset or null, returns all clicks over all time. */
    async clicksInLastNSeconds(nSeconds?: number | null): Promise<number> {
        const response = await this.dispatch({ type: "getClicks", nSeconds: nSeconds ?? null });
        if (response.type === 'error') {
            console.error("received error from worker:", response.msg);
            throw new Error(response.msg);
        } else if (response.type !== 'getClicks') {
            console.error("received improper response when getting clicks", response);
            throw new Error("received improper response type when getting clicks");
        }
        return response.count;
    }
}
