/*
 * Web wallpaper surface for Plasma 5 (Qt 5 QtWebEngine).
 *
 * Loaded on demand by main.qml when a WebUrl is configured, so QtWebEngine is
 * only required for users who actually use a web wallpaper. Requires the
 * `qml-module-qtwebengine` package.
 *
 * we-api-shim.js is injected before the wallpaper's own scripts so wallpapers
 * that wait for Wallpaper Engine's JS API start animating.
 */
import QtQuick 2.15
import QtWebEngine 1.1

WebEngineView {
    anchors.fill: parent
    url: wallpaper.configuration.WebUrl

    userScripts: [
        WebEngineScript {
            injectionPoint: WebEngineScript.DocumentCreation
            worldId: WebEngineScript.MainWorld
            runOnSubframes: true
            sourceUrl: Qt.resolvedUrl("we-api-shim.js")
        }
    ]
}
