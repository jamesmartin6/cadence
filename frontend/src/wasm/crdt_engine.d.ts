/* tslint:disable */
/* eslint-disable */

/**
 * The WASM-facing wrapper around [`Doc`], consumed directly from TypeScript.
 *
 * Operations are handed back and forth as `JsValue` (plain JSON-shaped objects via
 * `serde-wasm-bindgen`) so they can be sent over the WebSocket to the relay server
 * unchanged, and so remote operations received from the socket can be fed straight
 * back in without the frontend needing to know anything about the CRDT internals.
 */
export class CrdtDoc {
    free(): void;
    [Symbol.dispose](): void;
    applyRemote(op: any): void;
    delete(index: number): any;
    insert(index: number, ch: string): any;
    isEmpty(): boolean;
    len(): number;
    constructor(site_id: number);
    siteId(): number;
    toString(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_crdtdoc_free: (a: number, b: number) => void;
    readonly crdtdoc_applyRemote: (a: number, b: any) => [number, number];
    readonly crdtdoc_delete: (a: number, b: number) => any;
    readonly crdtdoc_insert: (a: number, b: number, c: number) => any;
    readonly crdtdoc_isEmpty: (a: number) => number;
    readonly crdtdoc_len: (a: number) => number;
    readonly crdtdoc_new: (a: number) => number;
    readonly crdtdoc_siteId: (a: number) => number;
    readonly crdtdoc_toString: (a: number) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
