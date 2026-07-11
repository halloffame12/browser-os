export class CapabilityAmbassador {
  constructor(kernel, wasmMem, onRemoteCap) {
    this.kernel = kernel;
    this.wasmMem = wasmMem;
    this.onRemoteCap = onRemoteCap || (() => {});
    this.peerConnections = new Map();
    this.dataChannels = new Map();
    this.secrets = new Map();
  }

  async createOffer(peerId, peerSecret) {
    const secretBytes = new TextEncoder().encode(peerSecret);
    this.secrets.set(peerId, secretBytes);

    const pc = new RTCPeerConnection({ iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] });
    this.peerConnections.set(peerId, pc);

    const dc = pc.createDataChannel('capability-bridge', { ordered: true });
    this.dataChannels.set(peerId, dc);
    this._setupChannel(dc, peerId);

    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);

    await new Promise(resolve => {
      if (pc.iceGatheringState === 'complete') resolve();
      else pc.onicegatheringstatechange = () => { if (pc.iceGatheringState === 'complete') resolve(); };
    });

    return {
      signaling: { sdp: pc.localDescription.sdp, type: pc.localDescription.type },
    };
  }

  async acceptOffer(peerId, peerSecret, offerSignaling) {
    const secretBytes = new TextEncoder().encode(peerSecret);
    this.secrets.set(peerId, secretBytes);

    const pc = new RTCPeerConnection({ iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] });
    this.peerConnections.set(peerId, pc);

    await pc.setRemoteDescription(new RTCSessionDescription(offerSignaling));

    const dc = pc.createDataChannel('capability-bridge', { ordered: true });
    this.dataChannels.set(peerId, dc);
    this._setupChannel(dc, peerId);

    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);

    await new Promise(resolve => {
      if (pc.iceGatheringState === 'complete') resolve();
      else pc.onicegatheringstatechange = () => { if (pc.iceGatheringState === 'complete') resolve(); };
    });

    return {
      signaling: { sdp: pc.localDescription.sdp, type: pc.localDescription.type },
    };
  }

  async completeHandshake(peerId, answerSignaling) {
    const pc = this.peerConnections.get(peerId);
    if (!pc) throw new Error(`No connection for peer ${peerId}`);
    await pc.setRemoteDescription(new RTCSessionDescription(answerSignaling));
  }

  async delegateCap(peerId, capSlot) {
    const dc = this.dataChannels.get(peerId);
    if (!dc || dc.readyState !== 'open') throw new Error('Data channel not open');

    const secret = this.secrets.get(peerId);
    if (!secret) throw new Error(`No secret for peer ${peerId}`);

    const secretInfo = this.wasmMem.writeBytes(secret);

    const tokenBytes = this.kernel.sys_delegate_cap(capSlot, secretInfo.offset, secretInfo.length);

    const tokenHex = Array.from(tokenBytes).map(b => b.toString(16).padStart(2, '0')).join('');
    dc.send(JSON.stringify({ type: 'capability', token: tokenHex }));
    return tokenHex;
  }

  _setupChannel(dc, peerId) {
    dc.onopen = () => {
      console.log(`[Ambassador] Data channel open for ${peerId}`);
    };
    dc.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.type === 'capability' && msg.token) {
          const tokenBytes = new Uint8Array(
            msg.token.match(/.{1,2}/g).map(b => parseInt(b, 16))
          );

          const secret = this.secrets.get(peerId);
          if (!secret) {
            console.warn(`[Ambassador] No secret for ${peerId}`);
            return;
          }

          const tokenInfo = this.wasmMem.writeBytes(tokenBytes);
          const secretInfo = this.wasmMem.writeBytes(secret);

          const peerIdHash = this._simpleHash(peerId);
          const result = this.kernel.sys_import_delegation(
            tokenInfo.offset, tokenInfo.length,
            secretInfo.offset, secretInfo.length,
            peerIdHash.lo, peerIdHash.hi
          );
          if (result >= 0) {
            this.onRemoteCap(peerId, result);
          } else {
            console.warn(`[Ambassador] Import failed: code ${result}`);
          }
        }
      } catch (e) {
        console.warn('[Ambassador] Failed to process message:', e);
      }
    };
    dc.onclose = () => {
      console.log(`[Ambassador] Data channel closed for ${peerId}`);
    };
  }

  _simpleHash(str) {
    let h1 = 0, h2 = 0;
    for (let i = 0; i < str.length; i++) {
      const c = str.charCodeAt(i);
      h1 = ((h1 << 5) - h1) + c;
      h1 |= 0;
      h2 = ((h2 << 7) - h2) + c;
      h2 |= 0;
    }
    return { lo: h1 >>> 0, hi: h2 >>> 0 };
  }

  close(peerId) {
    const dc = this.dataChannels.get(peerId);
    if (dc) dc.close();
    const pc = this.peerConnections.get(peerId);
    if (pc) pc.close();
    this.dataChannels.delete(peerId);
    this.peerConnections.delete(peerId);
    this.secrets.delete(peerId);
  }

  listRemoteProxies() {
    return this.kernel.sys_list_remote_proxies();
  }

  listDelegations() {
    return this.kernel.sys_list_delegations();
  }
}
