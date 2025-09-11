export const DEBUG_LOGS = true;
export function dlog(...s: any[]) {
    if (DEBUG_LOGS) {
        console.log(...s)
    }
}
