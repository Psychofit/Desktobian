/*
 * Web wallpaper surface for Plasma 5 (Qt 5 QtWebEngine).
 *
 * Loaded on demand by main.qml when a WebUrl is configured, so QtWebEngine is
 * only required for users who actually use a web wallpaper. Requires the
 * `qml-module-qtwebengine` package.
 */
import QtQuick 2.15
import QtWebEngine 1.10

WebEngineView {
    anchors.fill: parent
    url: wallpaper.configuration.WebUrl
    backgroundColor: "black"
    settings.showScrollBars: false
}
