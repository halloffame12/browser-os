export function createWasiShim(kernel) {
  if (!kernel.wasiTable) {
    kernel.wasiTable = {
      entries: new Array(256).fill(null),
      initRoot(capSlot) {
        this.entries[3] = { capSlot, rights: 0x3F, offset: 0n, isPreopened: true };
      },
      alloc(entry) {
        for (let fd = 4; fd < 256; fd++) {
          if (this.entries[fd] === null) {
            this.entries[fd] = entry;
            return fd;
          }
        }
        return null;
      },
      get(fd) { return this.entries[fd]; },
      free(fd) { this.entries[fd] = null; },
    };
  }

  return {
    "wasi_snapshot_preview1": {
      path_open(dirFd, dirFlags, pathPtr, pathLen, oflags,
                fsRightsBase, fsRightsInheriting, fdFlags, fdPtr) {
        const pathBytes = new Uint8Array(kernel.memory.buffer, pathPtr, pathLen);
        const path = new TextDecoder().decode(pathBytes);

        let rights = 0;
        if (fsRightsBase & 1n) rights |= 1;
        if (fsRightsBase & 2n) rights |= 2;

        const dirCap = kernel.wasiTable.get(dirFd);
        if (!dirCap) return 8;

        const result = kernel.sys_cap_open(BigInt(dirCap.capSlot), path, BigInt(rights));
        if (typeof result === 'number' && result < 0) return 54;

        const newFd = kernel.wasiTable.alloc({
          capSlot: Number(result),
          rights,
          offset: 0n,
          isPreopened: false,
        });
        if (newFd === null) return 23;

        new DataView(kernel.memory.buffer).setUint32(fdPtr, newFd, true);
        return 0;
      },

      fd_read(fd, iovsPtr, iovsLen, nreadPtr) {
        const entry = kernel.wasiTable.get(fd);
        if (!entry || (entry.rights & 1) === 0) return 9;

        const mem = kernel.memory.buffer;
        let totalRead = 0n;
        for (let i = 0; i < iovsLen; i++) {
          const view = new DataView(mem);
          const bufPtr = view.getUint32(iovsPtr + i * 8, true);
          const bufLen = view.getUint32(iovsPtr + i * 8 + 4, true);
          const buf = new Uint8Array(mem, bufPtr, bufLen);

          const result = kernel.sys_cap_read(BigInt(entry.capSlot));
          if (typeof result === 'string') {
            const bytes = new TextEncoder().encode(result);
            const n = Math.min(bytes.length, bufLen);
            buf.set(bytes.subarray(0, n));
            entry.offset += BigInt(n);
            totalRead += BigInt(n);
            break;
          }
          break;
        }
        new DataView(mem).setUint32(nreadPtr, Number(totalRead), true);
        return 0;
      },

      fd_write(fd, iovsPtr, iovsLen, nwrittenPtr) {
        const entry = kernel.wasiTable.get(fd);
        if (!entry || (entry.rights & 2) === 0) return 9;

        const mem = kernel.memory.buffer;
        let totalWritten = 0n;
        for (let i = 0; i < iovsLen; i++) {
          const view = new DataView(mem);
          const bufPtr = view.getUint32(iovsPtr + i * 8, true);
          const bufLen = view.getUint32(iovsPtr + i * 8 + 4, true);
          const bytes = new Uint8Array(mem, bufPtr, bufLen);

          const result = kernel.sys_cap_write(
            BigInt(entry.capSlot),
            BigInt(totalWritten),
            BigInt(bytes.length)
          );
          if (typeof result === 'number' && result < 0) return Number(result);
          totalWritten += BigInt(bytes.length);
        }
        new DataView(mem).setUint32(nwrittenPtr, Number(totalWritten), true);
        return 0;
      },

      fd_close(fd) {
        const entry = kernel.wasiTable.get(fd);
        if (!entry) return 9;
        kernel.sys_cap_destroy(BigInt(entry.capSlot));
        kernel.wasiTable.free(fd);
        return 0;
      },

      fd_seek(fd, offset, whence, newoffsetPtr) {
        const entry = kernel.wasiTable.get(fd);
        if (!entry) return 9;
        const mem = kernel.memory.buffer;
        switch (whence) {
          case 0: entry.offset = offset; break;
          case 1: entry.offset += offset; break;
          case 2: return 70; // ENOTSUP for now
        }
        new DataView(mem).setBigUint64(newoffsetPtr, entry.offset, true);
        return 0;
      },

      fd_prestat_get(fd, bufPtr) {
        if (fd !== 3) return 9;
        const mem = kernel.memory.buffer;
        new DataView(mem).setUint8(bufPtr, 0);
        new DataView(mem).setUint32(bufPtr + 4, 1, true);
        return 0;
      },

      fd_prestat_dir_name(fd, pathPtr, pathLen) {
        if (fd !== 3) return 9;
        new Uint8Array(kernel.memory.buffer)[pathPtr] = 0x2F;
        return 0;
      },

      fd_fdstat_get(fd, bufPtr) {
        const entry = kernel.wasiTable.get(fd);
        if (!entry) return 9;
        const mem = kernel.memory.buffer;
        const buf = new DataView(mem, bufPtr, 24);
        buf.setUint8(0, 2);
        buf.setUint16(2, entry.rights, true);
        buf.setUint16(4, entry.rights, true);
        return 0;
      },

      environ_sizes_get(countPtr, bufSizePtr) {
        new DataView(kernel.memory.buffer).setUint32(countPtr, 0, true);
        new DataView(kernel.memory.buffer).setUint32(bufSizePtr, 0, true);
        return 0;
      },

      environ_get(environPtr, environBufPtr) {
        return 0;
      },

      args_sizes_get(countPtr, bufSizePtr) {
        new DataView(kernel.memory.buffer).setUint32(countPtr, 1, true);
        new DataView(kernel.memory.buffer).setUint32(bufSizePtr, 1, true);
        return 0;
      },

      args_get(argvPtr, argvBufPtr) {
        new DataView(kernel.memory.buffer).setUint32(argvPtr, argvBufPtr, true);
        new Uint8Array(kernel.memory.buffer)[argvBufPtr] = 0;
        return 0;
      },

      proc_exit(code) {
        return 0;
      },
    },
  };
}
