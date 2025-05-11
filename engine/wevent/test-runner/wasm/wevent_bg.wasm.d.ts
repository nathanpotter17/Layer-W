/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const create_timer: () => number;
export const create_event_system: () => number;
export const run_demo: () => [number, number];
export const wasm_start: () => void;
export const __wbg_jstimer_free: (a: number, b: number) => void;
export const jstimer_new: () => number;
export const jstimer_elapsed_ms: (a: number) => number;
export const jstimer_reset: (a: number) => void;
export const __wbg_jswevent_free: (a: number, b: number) => void;
export const jswevent_new: () => number;
export const jswevent_update: (a: number) => void;
export const jswevent_has_events: (a: number) => number;
export const jswevent_event_count: (a: number) => number;
export const jswevent_push_custom_event: (a: number, b: number, c: number) => void;
export const jswevent_clear_events: (a: number) => void;
export const __wbindgen_export_0: WebAssembly.Table;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_start: () => void;
