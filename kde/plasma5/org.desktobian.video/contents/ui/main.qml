/*
 * Desktobian — native Plasma 5 wallpaper plugin (Qt 5).
 *
 * Renders a looping video/GIF (QtMultimedia) or a web wallpaper (QtWebEngine)
 * inside plasmashell's wallpaper layer, so desktop icons stay visible on top.
 *
 * The web part lives in a separate WebWallpaper.qml, loaded on demand, so that
 * QtWebEngine is only required when a web wallpaper is actually used.
 */
import QtQuick 2.15
import QtMultimedia 5.15

Item {
    id: root
    anchors.fill: parent

    // Config values from contents/config/main.xml via `wallpaper.configuration`.
    readonly property url videoUrl: wallpaper.configuration.VideoUrl
    readonly property url webUrl: wallpaper.configuration.WebUrl
    readonly property bool isWeb: root.webUrl.toString().length > 0

    // Backdrop shown before the first frame / when nothing is set.
    Rectangle {
        anchors.fill: parent
        color: "black"
    }

    // --- Web wallpaper (loaded by URL so QtWebEngine isn't a hard dep) -------
    Loader {
        anchors.fill: parent
        active: root.isWeb
        source: root.isWeb ? Qt.resolvedUrl("WebWallpaper.qml") : ""
    }

    // --- Video wallpaper ----------------------------------------------------
    Loader {
        anchors.fill: parent
        active: !root.isWeb
        sourceComponent: videoComponent
    }

    Component {
        id: videoComponent

        Item {
            anchors.fill: parent
            readonly property int fillModeIndex: wallpaper.configuration.FillMode

            MediaPlayer {
                id: player
                source: root.videoUrl
                autoPlay: true
                loops: wallpaper.configuration.Loop ? MediaPlayer.Infinite : 1
                muted: wallpaper.configuration.Muted
                volume: wallpaper.configuration.Volume / 100.0
            }

            VideoOutput {
                anchors.fill: parent
                source: player
                fillMode: parent.fillModeIndex === 0 ? VideoOutput.Stretch
                        : parent.fillModeIndex === 1 ? VideoOutput.PreserveAspectFit
                                                     : VideoOutput.PreserveAspectCrop
            }
        }
    }
}
