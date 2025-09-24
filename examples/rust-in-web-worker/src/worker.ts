declare var self: Worker;

import init, { initDb, click, clicksSince } from '../../../ts/gen/riww';
import type { InitResponse, WorkerRequest, WorkerResponse } from './messages';

await init();
await initDb("click-tracker.1.redb");

self.addEventListener("message", (event: MessageEvent<WorkerRequest>) => {
    const msg = event.data;
    const id = msg.id;

    try {
        switch (msg.body.type) {
            case 'click': {
                click(Date.now());
                postMessage({ id, type: 'click' } as WorkerResponse)
                break;
            }

            case 'getClicks': {
                var since: number | null = null;
                if (msg.body.nSeconds) {
                    since = Date.now() - (msg.body.nSeconds * 1000);
                }
                const count = clicksSince(since);
                postMessage({ id, type: 'getClicks', count } as WorkerResponse);
                break;
            }

            default: {
                console.error("worker: received unexpected event", msg);
                throw new Error("worker received unexpected event");
            }

        }
    } catch (err) {
        console.error("Worker error:", err);
        postMessage({ id, type: 'error', msg: String(err) } as WorkerResponse);
    }
});

self.postMessage({ type: "init" } as InitResponse);
