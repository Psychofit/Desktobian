/*
 * Web wallpaper surface for Plasma 6 (Qt 6 QtWebEngine).
 *
 * Loaded on demand by main.qml when a WebUrl is configured, so QtWebEngine is
 * only required for users who actually use a web wallpaper. Requires the
 * `qml6-module-qtwebengine` package.
 *
 * we-api-shim.js is injected before the wallpaper's own scripts so wallpapers
 * that wait for Wallpaper Engine's JS API start animating.
 */
import QtQuick
import QtWebEngine

WebEngineView {
    id: view
    anchors.fill: parent
    url: wallpaper.configuration.WebUrl

    WebEngineScript {
        id: weShim
        injectionPoint: WebEngineScript.DocumentCreation
        worldId: WebEngineScript.MainWorld
        runOnSubframes: true
        sourceUrl: Qt.resolvedUrl("we-api-shim.js")
    }

    Component.onCompleted: view.userScripts.insert(weShim)
}
