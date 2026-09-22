// The browser demo keeps its run history in localStorage. This is the whole
// bridge: three functions the Rust side imports (see src/profile.rs), moving
// UTF-8 bytes in and out of wasm memory. `wasm_memory` comes from gl.js.
"use strict";
var LAST_SIGNAL_KEY = "the-last-signal.profile";
var last_signal_pending = null;
function last_signal_read_store() {
  try {
    var v = window.localStorage.getItem(LAST_SIGNAL_KEY);
    return v === null ? null : new TextEncoder().encode(v);
  } catch (e) {
    return null;
  }
}
miniquad_add_plugin({
  register_plugin: function (importObject) {
    importObject.env.last_signal_store_len = function () {
      last_signal_pending = last_signal_read_store();
      return last_signal_pending === null ? 0 : last_signal_pending.length;
    };
    importObject.env.last_signal_store_read = function (ptr, cap) {
      var src = last_signal_pending || last_signal_read_store();
      last_signal_pending = null;
      if (src === null) return 0;
      var n = Math.min(cap, src.length);
      new Uint8Array(wasm_memory.buffer, ptr, n).set(src.subarray(0, n));
      return n;
    };
    importObject.env.last_signal_store_write = function (ptr, len) {
      try {
        var bytes = new Uint8Array(wasm_memory.buffer, ptr, len);
        window.localStorage.setItem(LAST_SIGNAL_KEY, new TextDecoder().decode(bytes));
      } catch (e) {}
    };
  },
  version: 1,
  name: "last_signal_storage",
});
