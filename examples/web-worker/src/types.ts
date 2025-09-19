export type Message = {
    id: number;
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
        typeof value.offset === "bigint" &&
        typeof value.id === "number"
    )
}

export type Response = Uint8Array;
