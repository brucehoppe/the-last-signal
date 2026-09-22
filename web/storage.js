// The browser demo keeps its run history in localStorage. This is the whole
// bridge: two functions the Rust side imports (see src/profile.rs).
"use strict";
var LAST_SIGNAL_KEY = "the-last-signal.profile";
miniquad_add_plugin({
  register_plugin: function (importObject) {
    importObject.env.last_signal_store_get = function () {
      try {
        var v = window.localStorage.getItem(LAST_SIGNAL_KEY);
        return js_object(v === null ? null : v);
      } catch (e) {
        return js_object(null);
      }
    };
    importObject.env.last_signal_store_set = function (value) {
      try {
        window.localStorage.setItem(LAST_SIGNAL_KEY, get_js_object(value));
      } catch (e) {}
    };
  },
  version: 1,
  name: "last_signal_storage",
});
