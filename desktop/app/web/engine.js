// Loads the Rust game engine (WebAssembly) and exposes it as window.AscensionEngine.
// The engine binary is embedded below by `cargo xtask frontend`.
(function () {
  'use strict';
  const WASM_BASE64 = '__ENGINE_WASM_BASE64__';

  window.AscensionEngineReady = (async () => {
    const bytes = Uint8Array.from(atob(WASM_BASE64), (c) => c.charCodeAt(0));
    const { instance } = await WebAssembly.instantiate(bytes, {
      env: { tz_offset_min: (ms) => -new Date(ms).getTimezoneOffset() },
    });
    const ex = instance.exports;
    const enc = new TextEncoder();
    const dec = new TextDecoder();

    function call(req) {
      const input = enc.encode(JSON.stringify(req));
      const ptr = ex.alloc(input.length);
      new Uint8Array(ex.memory.buffer, ptr, input.length).set(input);
      const out = ex.call(ptr, input.length);
      ex.dealloc(ptr, input.length);
      const len = new DataView(ex.memory.buffer).getUint32(out, true);
      const text = dec.decode(new Uint8Array(ex.memory.buffer, out + 4, len));
      ex.dealloc(out, len + 4);
      const reply = JSON.parse(text);
      if (reply.error) throw new Error(reply.error);
      return reply.ok;
    }

    window.AscensionEngine = {
      version: call({ op: 'version' }),
      compute: (state, today) => call({ op: 'compute', state, today }),
      targets: (profile) => call({ op: 'targets', state: profile }),
      importIcs: (state, ics, today, now) => call({ op: 'import_ics', state, ics, today, now }),
    };
    return window.AscensionEngine;
  })().catch((e) => {
    // The UI falls back to its JavaScript engine if this ever fails.
    console.error('Rust engine failed to load', e);
    return null;
  });
})();
