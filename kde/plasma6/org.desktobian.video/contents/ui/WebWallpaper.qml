/*
 * Web wallpaper surface for Plasma 6 (Qt 6 QtWebEngine).
 *
 * Loaded on demand by main.qml when a WebUrl is configured, so QtWebEngine is
 * only required for users who actually use a web wallpaper. Requires the
 * `qml6-module-qtwebengine` package.
 */
import QtQuick
import QtWebEngine

WebEngineView {
    anchors.fill: parent
    url: wallpaper.configuration.WebUrl
    backgroundColor: "black"
    settings.showScrollBars: false
}
