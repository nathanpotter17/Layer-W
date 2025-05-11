/* tslint:disable */
/* eslint-disable */
export class Walloc {
  private constructor();
  free(): void;
  static new(): Walloc;
  get_memory_view(offset: number, length: number): Uint8Array;
  read_u32(offset: number): number;
  write_u32(offset: number, value: number): void;
  memory_stats(): object;
  allocate(size: number): number;
  free(offset: number): void;
  copy_from_js(offset: number, data: Uint8Array): void;
  copy_to_js(offset: number, length: number): Uint8Array;
  realloc(offset: number, old_size: number, new_size: number): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_walloc_free: (a: number, b: number) => void;
  readonly walloc_new: () => number;
  readonly walloc_read_u32: (a: number, b: number) => [number, number, number];
  readonly walloc_write_u32: (a: number, b: number, c: number) => [number, number];
  readonly walloc_memory_stats: (a: number) => any;
  readonly walloc_allocate: (a: number, b: number) => number;
  readonly walloc_free: (a: number, b: number) => void;
  readonly walloc_copy_from_js: (a: number, b: number, c: any) => [number, number];
  readonly walloc_copy_to_js: (a: number, b: number, c: number) => [number, number, number];
  readonly walloc_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly walloc_get_memory_view: (a: number, b: number, c: number) => [number, number, number];
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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
