/* tslint:disable */
/* eslint-disable */
/**
 * JavaScript-friendly Engine wrapper
 */
export class JsEngine {
  free(): void;
  constructor();
  start(): void;
  stop(): void;
  is_running(): boolean;
  update(): void;
  is_button_pressed(gamepad_id: number, button_name: string): boolean;
  get_elapsed_time(): number;
  has_events(): boolean;
  /**
   * Shows all current events in the queue without consuming them
   */
  show_events(): Array<any>;
  /**
   * Process internal engine events (DO NOT call this in normal usage)
   */
  process_internal_events(): void;
}
/**
 * JavaScript-friendly event structure
 */
export class JsEvent {
  private constructor();
  free(): void;
  readonly event_type: string;
  readonly timestamp: number;
  readonly data: any;
  readonly button_id: string | undefined;
  readonly gamepad_id: number | undefined;
}
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
export class JsWInput {
  free(): void;
  constructor();
  update(): void;
  is_button_pressed(gamepad_id: number, button_name: string): boolean;
  get_elapsed_ms(): number;
  has_events(): boolean;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_jswinput_free: (a: number, b: number) => void;
  readonly jswinput_new: () => number;
  readonly jswinput_update: (a: number) => void;
  readonly jswinput_is_button_pressed: (a: number, b: number, c: number, d: number) => number;
  readonly jswinput_get_elapsed_ms: (a: number) => number;
  readonly jswinput_has_events: (a: number) => number;
  readonly __wbg_jsevent_free: (a: number, b: number) => void;
  readonly jsevent_event_type: (a: number) => [number, number];
  readonly jsevent_timestamp: (a: number) => number;
  readonly jsevent_data: (a: number) => any;
  readonly jsevent_button_id: (a: number) => [number, number];
  readonly jsevent_gamepad_id: (a: number) => [number, number];
  readonly __wbg_jsengine_free: (a: number, b: number) => void;
  readonly jsengine_new: () => [number, number, number];
  readonly jsengine_start: (a: number) => void;
  readonly jsengine_stop: (a: number) => void;
  readonly jsengine_is_running: (a: number) => number;
  readonly jsengine_update: (a: number) => [number, number];
  readonly jsengine_is_button_pressed: (a: number, b: number, c: number, d: number) => number;
  readonly jsengine_get_elapsed_time: (a: number) => number;
  readonly jsengine_has_events: (a: number) => number;
  readonly jsengine_show_events: (a: number) => any;
  readonly jsengine_process_internal_events: (a: number) => void;
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
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
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
