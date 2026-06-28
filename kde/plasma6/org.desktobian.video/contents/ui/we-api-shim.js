/*
 * Minimal Wallpaper Engine web-API shim (Desktobian).
 *
 * Injected before a web wallpaper's own scripts so wallpapers that wait for
 * Wallpaper Engine's JS API actually start animating. It provides no-op/default
 * implementations of the functions WE wallpapers expect, and nudges the
 * wallpaper's property listeners with defaults once the page loads.
 *
 * This is a best-effort compatibility layer: audio is fed as silence (we don't
 * capture desktop audio yet), and properties are applied with their defaults.
 */
(function () {
  if (window.__desktobianWeShim) return;
  window.__desktobianWeShim = true;

  // --- Audio-reactive wallpapers -----------------------------------------
  var audioCallback = null;
  window.wallpaperRegisterAudioListener = function (cb) {
    audioCallback = cb;
  };
  // 128 samples (64 left + 64 right). Silence for now.
  var silence = new Array(128).fill(0);
  setInterval(function () {
    if (typeof audioCallback === "function") {
      try {
        audioCallback(silence);
      } catch (e) {}
    }
  }, 33);

  // --- Other WE registration hooks (no-ops) ------------------------------
  var noop = function () {};
  window.wallpaperRegisterMediaStatusListener = noop;
  window.wallpaperRegisterMediaPropertiesListener = noop;
  window.wallpaperRegisterMediaThumbnailListener = noop;
  window.wallpaperRegisterMediaTimelineListener = noop;
  window.wallpaperRequestRandomFileForProperty = noop;
  window.wallpaperPropertyListener = window.wallpaperPropertyListener || {};

  // --- Forwarded pointer interaction (Desktobian) ------------------------
  // The Plasma plugin keeps the web view input-passive (so the desktop's
  // right-click menu keeps working) and instead forwards cursor movement and
  // middle-clicks here as synthetic DOM mouse events, so interactive wallpapers
  // (cursor parallax, click handlers) still react to the pointer.
  var buttonsBit = { 0: 1, 1: 4, 2: 2 }; // DOM "buttons" bitmask per button id
  window.__desktobianDispatchMouse = function (type, x, y, button) {
    try {
      var target = document.elementFromPoint(x, y) || document.body || document;
      var ev = new MouseEvent(type, {
        bubbles: true,
        cancelable: true,
        view: window,
        clientX: x,
        clientY: y,
        screenX: x,
        screenY: y,
        button: button,
        buttons: type === "mousedown" ? buttonsBit[button] || 0 : 0,
      });
      target.dispatchEvent(ev);
    } catch (e) {}
  };

  // --- Kick off the wallpaper with default properties --------------------
  function applyDefaults() {
    var l = window.wallpaperPropertyListener;
    if (!l) return;
    try {
      if (l.applyGeneralProperties) l.applyGeneralProperties({ fps: 60 });
    } catch (e) {}
    try {
      if (l.applyUserProperties) l.applyUserProperties({});
    } catch (e) {}
    try {
      if (l.setPaused) l.setPaused(false);
    } catch (e) {}
  }
  if (document.readyState === "complete") {
    applyDefaults();
  } else {
    window.addEventListener("load", applyDefaults);
  }
})();
