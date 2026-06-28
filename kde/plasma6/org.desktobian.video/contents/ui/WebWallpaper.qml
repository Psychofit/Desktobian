/*
 * Web wallpaper surface for Plasma 6 (Qt 6 QtWebEngine).
 *
 * Loaded on demand by main.qml when a WebUrl is configured, so QtWebEngine is
 * only required for users who actually use a web wallpaper. Requires the
 * `qml6-module-qtwebengine` package.
 *
 * we-api-shim.js is injected before the wallpaper's own scripts so wallpapers
 * that wait for Wallpaper Engine's JS API start animating.
 *
 * Input model (controlled by the "Web interaction" config option):
 *  - off (default): the view is input-passive, so left/right clicks fall through
 *    to the Plasma desktop (right-click opens the normal containment menu).
 *    Movement and left/middle-clicks are forwarded to the page as synthetic DOM
 *    events (see we-api-shim.js) for best-effort parallax/clicks.
 *  - on: the view takes real (trusted) mouse input so interactive wallpapers
 *    respond fully; the right button is swallowed (no browser/desktop menu).
 */
import QtQuick
import QtWebEngine

Item {
    anchors.fill: parent

    WebEngineView {
        id: view
        anchors.fill: parent
        url: wallpaper.configuration.WebUrl

        // Input model depends on the "Web interaction" config option:
        //  - off (default): input-passive, so left/right clicks fall through to
        //    the Plasma desktop (right-click opens the containment menu).
        //    Movement and left/middle-clicks are forwarded via JS (MouseArea).
        //  - on: take native mouse input, so wallpapers that need real clicks /
        //    parallax work fully — but the desktop right-click menu is then
        //    unavailable while this wallpaper is active.
        enabled: wallpaper.configuration.MouseInteraction

        WebEngineScript {
            id: weShim
            injectionPoint: WebEngineScript.DocumentCreation
            worldId: WebEngineScript.MainWorld
            runOnSubframes: true
            sourceUrl: Qt.resolvedUrl("we-api-shim.js")
        }

        Component.onCompleted: view.userScripts.insert(weShim)
    }

    // Mouse handling adapts to the "Web interaction" option:
    //  - off (view passive): forward movement + left/middle-clicks to the page
    //    as synthetic events, and let the right button fall through to Plasma's
    //    desktop menu.
    //  - on (view native): accept only the right button and swallow it, so it
    //    neither triggers the wallpaper nor shows a browser menu; left/middle/
    //    movement are not accepted here and reach the view as real (trusted)
    //    input, which interactive wallpapers actually respond to.
    MouseArea {
        anchors.fill: parent
        readonly property bool interactive: view.enabled
        acceptedButtons: interactive ? Qt.RightButton
                                     : (Qt.LeftButton | Qt.MiddleButton)
        hoverEnabled: !interactive
        property double lastMove: 0

        function send(type, x, y, button) {
            view.runJavaScript(
                "window.__desktobianDispatchMouse && window.__desktobianDispatchMouse('"
                + type + "'," + Math.round(x) + "," + Math.round(y) + "," + button + ")");
        }
        // DOM button id: 0 = left, 1 = middle.
        function domButton(b) {
            return b === Qt.MiddleButton ? 1 : 0;
        }

        onPositionChanged: (mouse) => {
            if (interactive)
                return;
            var now = Date.now();
            if (now - lastMove < 16) // throttle to ~60 Hz
                return;
            lastMove = now;
            send("move", mouse.x, mouse.y, 0);
        }
        onPressed: (mouse) => {
            if (interactive)
                return; // right button swallowed; nothing to forward
            send("down", mouse.x, mouse.y, domButton(mouse.button));
        }
        onReleased: (mouse) => {
            if (interactive)
                return;
            send("up", mouse.x, mouse.y, domButton(mouse.button));
        }
    }
}
