/* tslint:disable */
/* eslint-disable */

export class WasmSimulation {
    free(): void;
    [Symbol.dispose](): void;
    height(): number;
    constructor(width: number, height: number);
    set_particle(x: number, y: number, element_id: number): void;
    tick(): void;
    width(): number;
}

export function create_simulation(width: number, height: number): WasmSimulation;

export function main_js(): void;

/**
 * Run a simple benchmark in WASM
 */
export function run_tick_test(width: number, height: number, ticks: number): number;

/**
 * Exported start_sim per spec
 */
export function start_sim(canvas_id: string): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly create_simulation: (a: number, b: number) => number;
    readonly run_tick_test: (a: number, b: number, c: number) => number;
    readonly start_sim: (a: number, b: number) => [number, number];
    readonly main_js: () => void;
    readonly __wbg_wasmsimulation_free: (a: number, b: number) => void;
    readonly wasmsimulation_height: (a: number) => number;
    readonly wasmsimulation_new: (a: number, b: number) => number;
    readonly wasmsimulation_set_particle: (a: number, b: number, c: number, d: number) => void;
    readonly wasmsimulation_tick: (a: number) => void;
    readonly wasmsimulation_width: (a: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
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
