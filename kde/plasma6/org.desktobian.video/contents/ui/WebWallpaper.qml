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
 * Input model: the WebEngineView is kept input-passive (enabled:false) so left-
 * and right-clicks fall through to the Plasma desktop — right-click still opens
 * the normal containment menu instead of QtWebEngine's browser menu. Cursor
 * movement and middle-clicks are forwarded to the page as synthetic DOM mouse
 * events (see we-api-shim.js), so interactive wallpapers still react to the
 * pointer without stealing the desktop's right-click menu.
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

    // Forward cursor movement + left/middle-clicks to the page. The right button
    // is deliberately not accepted, so it falls through to the Plasma desktop and
    // opens the normal containment menu.
    MouseArea {
        anchors.fill: parent
        // Only forward while the view is passive; in native-input mode the view
        // handles the mouse itself, so this must not intercept events.
        enabled: !view.enabled
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        hoverEnabled: true
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
            var now = Date.now();
            if (now - lastMove < 16) // throttle to ~60 Hz
                return;
            lastMove = now;
            send("move", mouse.x, mouse.y, 0);
        }
        onPressed: (mouse) => send("down", mouse.x, mouse.y, domButton(mouse.button))
        onReleased: (mouse) => send("up", mouse.x, mouse.y, domButton(mouse.button))
    }
}
