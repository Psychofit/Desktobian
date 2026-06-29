/*
 * Pure helpers for the in-app web wallpaper property editor.
 *
 * No DOM or Tauri dependencies, so they can be unit-tested under node. Loaded
 * before main.js; exposes a `Props` global. The override map is a sparse JSON
 * object `{ name: value }` of values the user changed away from the project
 * defaults — the same contract the KDE plugin's WebProperties entry uses.
 */
(function (root) {
  function has(o, k) {
    return Object.prototype.hasOwnProperty.call(o, k);
  }

  function parseOverrides(text) {
    try {
      var o = JSON.parse(text || "{}");
      return o && typeof o === "object" ? o : {};
    } catch (e) {
      return {};
    }
  }

  function valueFor(overrides, name, def) {
    return has(overrides, name) ? overrides[name] : def;
  }

  // Return the overrides JSON after setting name=value, dropping the entry when
  // it matches the project default so the stored map stays sparse.
  function withOverride(text, name, value, def) {
    var o = parseOverrides(text);
    if (value === def) {
      delete o[name];
    } else {
      o[name] = value;
    }
    return JSON.stringify(o);
  }

  function clampByte(n) {
    return Math.max(0, Math.min(255, Math.round(isNaN(n) ? 0 : n)));
  }

  // WE colours are "r g b" floats in 0..1.
  function colorToRgb(str) {
    var p = String(str == null ? "" : str)
      .trim()
      .split(/\s+/)
      .map(parseFloat);
    return {
      r: clampByte((p[0] || 0) * 255),
      g: clampByte((p[1] || 0) * 255),
      b: clampByte((p[2] || 0) * 255),
    };
  }

  function hex2(n) {
    var s = clampByte(n).toString(16);
    return s.length < 2 ? "0" + s : s;
  }

  // "r g b" floats -> "#rrggbb" for an <input type="color">.
  function colorToHex(str) {
    var c = colorToRgb(str);
    return "#" + hex2(c.r) + hex2(c.g) + hex2(c.b);
  }

  // "#rrggbb" -> tidy "r g b" float string.
  function hexToColorString(hex) {
    var m = /^#?([0-9a-fA-F]{6})$/.exec(String(hex == null ? "" : hex).trim());
    if (!m) {
      return "0 0 0";
    }
    var n = parseInt(m[1], 16);
    var parts = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
    return parts
      .map(function (v) {
        return parseFloat((v / 255).toFixed(6)).toString();
      })
      .join(" ");
  }

  root.Props = {
    parseOverrides: parseOverrides,
    valueFor: valueFor,
    withOverride: withOverride,
    colorToRgb: colorToRgb,
    colorToHex: colorToHex,
    hexToColorString: hexToColorString,
  };
})(typeof window !== "undefined" ? window : globalThis);
