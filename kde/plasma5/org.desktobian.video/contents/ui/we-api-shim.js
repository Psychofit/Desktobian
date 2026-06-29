/*
 * Minimal Wallpaper Engine web-API shim (Desktobian).
 *
 * Injected before a web wallpaper's own scripts so wallpapers that wait for
 * Wallpaper Engine's JS API actually start animating. It provides no-op/default
 * implementations of the functions WE wallpapers expect, fetches the wallpaper's
 * real default properties from its project.json, and hands them to the
 * wallpaper's property listener once it has registered.
 *
 * This is a best-effort compatibility layer: audio is fed as silence (we don't
 * capture desktop audio yet), but a wallpaper's configured colours, sliders and
 * toggles (its project.json `general.properties` defaults) are applied for real.
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

  // --- Default wallpaper properties from project.json --------------------
  // Wallpaper Engine web wallpapers read their user-configurable settings
  // (colours, sliders, combos, toggles, …) from the `general.properties` block
  // of the project's project.json, delivered through
  // wallpaperPropertyListener.applyUserProperties() in the shape
  // `{ propName: { value: <default> }, … }`. We pull those defaults out and pass
  // them through, so a wallpaper renders with its intended look instead of
  // whatever it falls back to when no properties arrive.
  function userPropertiesFromProject(project) {
    var out = {};
    var props = project && project.general && project.general.properties;
    if (!props || typeof props !== "object") return out;
    for (var name in props) {
      if (!Object.prototype.hasOwnProperty.call(props, name)) continue;
      var def = props[name];
      // Skip pure UI entries (e.g. type "text" group headers) that carry no
      // value; only forward properties that actually have a default.
      if (def && typeof def === "object" && "value" in def) {
        out[name] = { value: def.value };
      }
    }
    return out;
  }

  // Layer the user's customised values (set in the Plasma config UI and injected
  // as `window.__desktobianUserPropertyOverrides`, a plain `{ name: value }`
  // map) on top of the project defaults. Overrides win and may add properties.
  function withOverrides(defaults) {
    var out = {};
    for (var k in defaults) {
      if (Object.prototype.hasOwnProperty.call(defaults, k)) out[k] = defaults[k];
    }
    var overrides = window.__desktobianUserPropertyOverrides;
    if (overrides && typeof overrides === "object") {
      for (var name in overrides) {
        if (!Object.prototype.hasOwnProperty.call(overrides, name)) continue;
        out[name] = { value: overrides[name] };
      }
    }
    return out;
  }

  // Fetch project.json relative to the wallpaper page. QtWebEngine can't fetch()
  // over file://, so this only succeeds when the page is served through the
  // Desktobian localhost http server (the default for the Plasma plugin).
  // Resolves to {} on any failure (no file, file:// origin, malformed JSON) so
  // the wallpaper still starts — just with defaults/overrides only, as before.
  var defaultsPromise = fetch("project.json", { cache: "no-store" })
    .then(function (r) {
      return r.ok ? r.json() : null;
    })
    .then(function (p) {
      return userPropertiesFromProject(p);
    })
    .catch(function () {
      return {};
    });

  // Re-merge defaults + current overrides and push them to the wallpaper.
  // Exposed so the Plasma plugin can re-apply live when the user edits a
  // property in the config UI, without reloading the page.
  window.__desktobianApplyProperties = function () {
    defaultsPromise.then(function (defaults) {
      var l = window.wallpaperPropertyListener;
      if (!l || !l.applyUserProperties) return;
      try {
        l.applyUserProperties(withOverrides(defaults));
      } catch (e) {}
    });
  };

  // --- Kick off the wallpaper with its real properties -------------------
  // Wallpapers register wallpaperPropertyListener from their own scripts, which
  // may run after our 'load' handler fires. Retry briefly until the listener
  // exposes a real handler, then apply the properties once.
  function kickOff() {
    defaultsPromise.then(function (defaults) {
      var userProps = withOverrides(defaults);
      var attempts = 0;
      (function attempt() {
        var l = window.wallpaperPropertyListener;
        if (l && (l.applyUserProperties || l.applyGeneralProperties)) {
          try {
            if (l.applyGeneralProperties)
              l.applyGeneralProperties({ fps: 60 });
          } catch (e) {}
          try {
            if (l.applyUserProperties) l.applyUserProperties(userProps);
          } catch (e) {}
          try {
            if (l.setPaused) l.setPaused(false);
          } catch (e) {}
          return;
        }
        if (++attempts > 40) {
          // ~4s elapsed; give up waiting but still try to unpause.
          if (l && l.setPaused) {
            try {
              l.setPaused(false);
            } catch (e) {}
          }
          return;
        }
        setTimeout(attempt, 100);
      })();
    });
  }
  if (document.readyState === "complete") {
    kickOff();
  } else {
    window.addEventListener("load", kickOff);
  }
})();
