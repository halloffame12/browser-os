/* tslint:disable */
/* eslint-disable */

export function boot(current_time_ms: bigint): string;

export function cap_destroy(cap_slot: number): number;

export function cap_get_root(): number;

export function cap_info(cap_slot: number): string;

export function cap_list(): string;

export function cap_mint(parent_slot: number, object_type: number, object_id: number, rights_flags: number): number;

export function cap_revoke(cap_slot: number): number;

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

export function sys_cap_create(dir_cap_slot: number, path: string, is_dir: boolean): number;

export function sys_cap_exists(dir_cap_slot: number, path: string): number;

export function sys_cap_list(dir_cap_slot: number, path: string): string;

export function sys_cap_open(dir_cap_slot: number, path: string, flags: number): number;

export function sys_cap_read(file_cap_slot: number): string;

export function sys_cap_spawn(parent_cap_slot: number): number;

export function sys_cap_write(file_cap_slot: number, data_offset: number, data_len: number): number;

export function sys_delegate_cap(cap_slot: number, peer_key_ptr: number, peer_key_len: number): Uint8Array;

export function sys_import_delegation(token_ptr: number, token_len: number, peer_key_ptr: number, peer_key_len: number, peer_id_lo: number, peer_id_hi: number): number;

export function sys_list_delegations(): string;

export function sys_list_remote_proxies(): string;

export function sys_revoke_delegation(delegation_id_lo: number, delegation_id_hi: number): number;

export function sys_snapshot_deserialize(data: Uint8Array): number;

export function sys_snapshot_list(): string[];

export function sys_snapshot_prepare(): number;

export function sys_snapshot_serialize(): Uint8Array;

export function uname(): string;

export function update_time(current_time_ms: bigint): void;

export function wasi_fd_to_cap(fd: number): number;

export function wasi_get_root_fd(): number;

export function wasi_init_root(): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly boot: (a: number, b: bigint) => void;
    readonly cap_destroy: (a: number) => number;
    readonly cap_get_root: () => number;
    readonly cap_info: (a: number, b: number) => void;
    readonly cap_list: (a: number) => void;
    readonly cap_mint: (a: number, b: number, c: number, d: number) => number;
    readonly cap_revoke: (a: number) => number;
    readonly fs_cat: (a: number, b: number, c: number) => void;
    readonly fs_close: (a: number) => number;
    readonly fs_create: (a: number, b: number, c: number) => number;
    readonly fs_exists: (a: number, b: number) => number;
    readonly fs_list: (a: number, b: number, c: number) => void;
    readonly fs_open: (a: number, b: number, c: number, d: number) => number;
    readonly fs_read: (a: number, b: number, c: number) => void;
    readonly fs_write: (a: number, b: number, c: number) => number;
    readonly get_uptime: () => bigint;
    readonly handle_command: (a: number, b: number, c: number) => void;
    readonly process_spawn: (a: number) => number;
    readonly sys_cap_create: (a: number, b: number, c: number, d: number) => number;
    readonly sys_cap_exists: (a: number, b: number, c: number) => number;
    readonly sys_cap_list: (a: number, b: number, c: number, d: number) => void;
    readonly sys_cap_open: (a: number, b: number, c: number, d: number) => number;
    readonly sys_cap_read: (a: number, b: number) => void;
    readonly sys_cap_spawn: (a: number) => number;
    readonly sys_cap_write: (a: number, b: number, c: number) => number;
    readonly sys_delegate_cap: (a: number, b: number, c: number, d: number) => void;
    readonly sys_import_delegation: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly sys_list_delegations: (a: number) => void;
    readonly sys_list_remote_proxies: (a: number) => void;
    readonly sys_revoke_delegation: (a: number, b: number) => number;
    readonly sys_snapshot_deserialize: (a: number, b: number) => number;
    readonly sys_snapshot_list: (a: number) => void;
    readonly sys_snapshot_prepare: () => number;
    readonly sys_snapshot_serialize: (a: number) => void;
    readonly uname: (a: number) => void;
    readonly update_time: (a: bigint) => void;
    readonly wasi_fd_to_cap: (a: number) => number;
    readonly wasi_get_root_fd: () => number;
    readonly wasi_init_root: () => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
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
