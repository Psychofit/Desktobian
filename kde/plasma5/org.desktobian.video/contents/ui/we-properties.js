.pragma library
/*
 * Shared helpers for the Desktobian web-wallpaper property editor.
 *
 * Pure, stateless functions (no QML/Qt types) so they can be unit-tested under
 * node as well as imported by config.qml on Plasma 5 and 6. They turn a
 * Wallpaper Engine project.json into a flat, typed model the config UI can
 * render, and read/write the sparse "overrides" map persisted in the plugin's
 * WebProperties config entry.
 *
 * The overrides map is `{ propName: value }` and only stores values the user
 * has actually changed away from the project default, so a wallpaper author's
 * later default tweaks still flow through for untouched properties.
 */

function _has(obj, key) {
    return Object.prototype.hasOwnProperty.call(obj, key);
}

/*
 * Parse a project.json string into an ordered list of editable properties:
 *   { name, type, label, order, def, [min, max, step], [options] }
 *
 * Types are normalised to: bool | slider | combo | color | textinput. WE's
 * "int" is treated as a stepped slider. Entries without a value (e.g. "text"
 * group headers) are skipped. Returns [] on any problem.
 */
function parseProperties(jsonText) {
    var project;
    try {
        project = JSON.parse(jsonText);
    } catch (e) {
        return [];
    }
    var props = project && project.general && project.general.properties;
    if (!props || typeof props !== "object") {
        return [];
    }

    var list = [];
    for (var name in props) {
        if (!_has(props, name)) {
            continue;
        }
        var d = props[name];
        if (!d || typeof d !== "object" || !_has(d, "value")) {
            continue;
        }

        var type = String(d.type || "").toLowerCase();
        var item = {
            name: name,
            type: type,
            label: d.text || name,
            order: typeof d.order === "number" ? d.order : 1000,
            def: d.value
        };

        if (type === "int") {
            item.type = "slider";
            item.min = typeof d.min === "number" ? d.min : 0;
            item.max = typeof d.max === "number" ? d.max : 100;
            item.step = typeof d.step === "number" && d.step > 0 ? d.step : 1;
        } else if (type === "slider") {
            item.min = typeof d.min === "number" ? d.min : 0;
            item.max = typeof d.max === "number" ? d.max : 1;
            item.step = typeof d.step === "number" && d.step > 0 ? d.step : 0;
        } else if (type === "combo") {
            var opts = Array.isArray(d.options) ? d.options : [];
            item.options = opts.map(function (o) {
                var value = o ? o.value : undefined;
                var label = o && o.label !== undefined ? o.label : String(value);
                return { label: label, value: value };
            });
        }

        list.push(item);
    }

    list.sort(function (a, b) {
        if (a.order !== b.order) {
            return a.order - b.order;
        }
        return a.label < b.label ? -1 : (a.label > b.label ? 1 : 0);
    });
    return list;
}

/* Parse the persisted overrides JSON into a plain object ({} on failure). */
function parseOverrides(jsonText) {
    try {
        var o = JSON.parse(jsonText || "{}");
        return o && typeof o === "object" ? o : {};
    } catch (e) {
        return {};
    }
}

/* The effective value for a property: the user's override, else the default. */
function valueFor(overrides, name, def) {
    return _has(overrides, name) ? overrides[name] : def;
}

/*
 * Return the overrides JSON after setting name=value. Setting a value equal to
 * the project default removes the entry, keeping the stored map sparse.
 */
function withOverride(jsonText, name, value, def) {
    var o = parseOverrides(jsonText);
    if (value === def) {
        delete o[name];
    } else {
        o[name] = value;
    }
    return JSON.stringify(o);
}

/* WE colours are "r g b" floats in 0..1. Convert to 0..255 ints for the UI. */
function colorToRgb(str) {
    var parts = String(str === undefined || str === null ? "" : str)
        .trim()
        .split(/\s+/)
        .map(parseFloat);
    function ch(n) {
        var v = isNaN(n) ? 0 : n;
        return Math.max(0, Math.min(255, Math.round(v * 255)));
    }
    return { r: ch(parts[0]), g: ch(parts[1]), b: ch(parts[2]) };
}

/* Inverse of colorToRgb: 0..255 ints -> a tidy "r g b" float string. */
function rgbToColorString(r, g, b) {
    function f(n) {
        var v = Math.max(0, Math.min(1, (isNaN(n) ? 0 : n) / 255));
        return parseFloat(v.toFixed(6)).toString();
    }
    return [f(r), f(g), f(b)].join(" ");
}
