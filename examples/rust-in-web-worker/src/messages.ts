export type WorkerRequest = {
    id: number;
    body: WorkerRequestBody;
};

export type WorkerRequestBody =
    | { type: 'click' }
    | { type: 'getClicks'; nSeconds: number | null };

export type InitResponse = { type: 'init' };

export type WorkerResponse =
    | { id: number; type: 'click' }
    | { id: number; type: 'getClicks'; count: number }
    | { id: number; type: 'error', msg: string };
