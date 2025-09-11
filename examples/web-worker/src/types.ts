export type Message = {
    op: "store" | "load";
    offset: bigint;
    data?: Uint8Array;
    size?: number;
}

export function isMessage(value: any): value is Message {
    return (
        typeof value === "object" &&
        value !== null &&
        (value.op === "store" || value.op === "load") &&
        typeof value.offset === "bigint"
    )
}

export type Response = Uint8Array;
