/* tslint:disable */
/* eslint-disable */

export function boot(current_time_ms: bigint): string;

export function fs_cat(path: string): string;

export function fs_close(fd: number): number;

export function fs_create(path: string, is_dir: boolean): number;

export function fs_exists(path: string): number;

export function fs_list(path: string): string;

export function fs_open(path: string, mode: string): number;

export function fs_read(fd: number, size: number): string;

export function fs_write(fd: number, data: string): number;

export function get_uptime(): bigint;

export function handle_command(cmd: string): string;

export function process_spawn(parent_pid: number): number;

export function uname(): string;

export function update_time(current_time_ms: bigint): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly boot: (a: bigint) => [number, number];
    readonly fs_cat: (a: number, b: number) => [number, number];
    readonly fs_close: (a: number) => number;
    readonly fs_create: (a: number, b: number, c: number) => number;
    readonly fs_exists: (a: number, b: number) => number;
    readonly fs_list: (a: number, b: number) => [number, number];
    readonly fs_open: (a: number, b: number, c: number, d: number) => number;
    readonly fs_read: (a: number, b: number) => [number, number];
    readonly fs_write: (a: number, b: number, c: number) => number;
    readonly get_uptime: () => bigint;
    readonly handle_command: (a: number, b: number) => [number, number];
    readonly process_spawn: (a: number) => number;
    readonly uname: () => [number, number];
    readonly update_time: (a: bigint) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
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
