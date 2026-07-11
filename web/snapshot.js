export class SnapshotManager {
  constructor(kernel) {
    this.kernel = kernel;
  }

  async snapshot(tag = `snap-${Date.now()}`) {
    this.kernel.sys_snapshot_prepare();

    const kernelState = this.kernel.sys_snapshot_serialize();

    try {
      const root = await navigator.storage.getDirectory();
      const snapDir = await root.getDirectoryHandle('snapshots', { create: true });

      const kernFile = await snapDir.getFileHandle(`${tag}.kern`, { create: true });
      const kernWritable = await kernFile.createWritable();
      await kernWritable.write(kernelState);
      await kernWritable.close();

      const manifest = {
        tag,
        timestamp: Date.now(),
        kernelStateSize: kernelState.length,
      };
      const manifestFile = await snapDir.getFileHandle(`${tag}.json`, { create: true });
      const manWritable = await manifestFile.createWritable();
      await manWritable.write(JSON.stringify(manifest, null, 2));
      await manWritable.close();

      return tag;
    } catch (e) {
      throw new Error(`Snapshot failed: ${e.message}`);
    }
  }

  async restore(tag) {
    try {
      const root = await navigator.storage.getDirectory();
      const snapDir = await root.getDirectoryHandle('snapshots');

      const kernFile = await snapDir.getFileHandle(`${tag}.kern`);
      const kernBuffer = await kernFile.getFile().then(f => f.arrayBuffer());

      const result = this.kernel.sys_snapshot_deserialize(new Uint8Array(kernBuffer));
      if (result !== 0) {
        throw new Error(`Kernel deserialize returned ${result}`);
      }

      return true;
    } catch (e) {
      throw new Error(`Restore failed: ${e.message}`);
    }
  }

  async list() {
    try {
      const root = await navigator.storage.getDirectory();
      const snapDir = await root.getDirectoryHandle('snapshots');
      const tags = [];
      for await (const [name] of snapDir.entries()) {
        if (name.endsWith('.json')) {
          tags.push(name.replace('.json', ''));
        }
      }
      return tags;
    } catch {
      return [];
    }
  }
}
