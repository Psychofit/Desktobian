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

  // --- Suppress the browser's own right-click menu -----------------------
  // Stops Chromium's "Back / Reload / Save Image…" menu from appearing over the
  // desktop in native-input mode. Page-level contextmenu handlers still run.
  window.addEventListener(
    "contextmenu",
    function (e) {
      e.preventDefault();
    },
    true
  );

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
  // left/middle-clicks here, so interactive wallpapers (cursor parallax, click
  // handlers) still react to the pointer. We dispatch BOTH mouse and pointer
  // events because different wallpapers listen to different ones.
  //
  // type is "move" | "down" | "up"; button is the DOM button id
  // (0 = left, 1 = middle, 2 = right).
  var buttonsBit = { 0: 1, 1: 4, 2: 2 }; // DOM "buttons" bitmask per button id
  window.__desktobianDispatchMouse = function (type, x, y, button) {
    try {
      var target = document.elementFromPoint(x, y) || document.body || document;
      var init = {
        bubbles: true,
        cancelable: true,
        view: window,
        clientX: x,
        clientY: y,
        screenX: x,
        screenY: y,
        button: button,
        buttons: type === "down" ? buttonsBit[button] || 0 : 0,
      };
      var mouseType =
        type === "down" ? "mousedown" : type === "up" ? "mouseup" : "mousemove";
      target.dispatchEvent(new MouseEvent(mouseType, init));
      if (type === "up") target.dispatchEvent(new MouseEvent("click", init));
      if (typeof PointerEvent === "function") {
        var pinit = {};
        for (var k in init) pinit[k] = init[k];
        pinit.pointerId = 1;
        pinit.pointerType = "mouse";
        pinit.isPrimary = true;
        var pointerType =
          type === "down"
            ? "pointerdown"
            : type === "up"
            ? "pointerup"
            : "pointermove";
        target.dispatchEvent(new PointerEvent(pointerType, pinit));
      }
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
