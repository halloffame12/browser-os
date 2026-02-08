/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const boot: (a: bigint) => [number, number];
export const fs_cat: (a: number, b: number) => [number, number];
export const fs_close: (a: number) => number;
export const fs_create: (a: number, b: number, c: number) => number;
export const fs_exists: (a: number, b: number) => number;
export const fs_list: (a: number, b: number) => [number, number];
export const fs_open: (a: number, b: number, c: number, d: number) => number;
export const fs_read: (a: number, b: number) => [number, number];
export const fs_write: (a: number, b: number, c: number) => number;
export const get_uptime: () => bigint;
export const handle_command: (a: number, b: number) => [number, number];
export const process_spawn: (a: number) => number;
export const uname: () => [number, number];
export const update_time: (a: bigint) => void;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_start: () => void;
