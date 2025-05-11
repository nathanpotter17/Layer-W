/* tslint:disable */
/* eslint-disable */
export function create_timer(): JsTimer;
export function create_event_system(): JsWEvent;
export function run_demo(): string;
export function wasm_start(): void;
export class JsTimer {
  free(): void;
  constructor();
  elapsed_ms(): number;
  reset(): void;
}
export class JsWEvent {
  free(): void;
  constructor();
  update(): void;
  has_events(): boolean;
  event_count(): number;
  push_custom_event(name: string): void;
  clear_events(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly create_timer: () => number;
  readonly create_event_system: () => number;
  readonly run_demo: () => [number, number];
  readonly wasm_start: () => void;
  readonly __wbg_jstimer_free: (a: number, b: number) => void;
  readonly jstimer_new: () => number;
  readonly jstimer_elapsed_ms: (a: number) => number;
  readonly jstimer_reset: (a: number) => void;
  readonly __wbg_jswevent_free: (a: number, b: number) => void;
  readonly jswevent_new: () => number;
  readonly jswevent_update: (a: number) => void;
  readonly jswevent_has_events: (a: number) => number;
  readonly jswevent_event_count: (a: number) => number;
  readonly jswevent_push_custom_event: (a: number, b: number, c: number) => void;
  readonly jswevent_clear_events: (a: number) => void;
  readonly __wbindgen_export_0: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
