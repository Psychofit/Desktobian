/*
 * Desktobian — a native Plasma 6 wallpaper plugin.
 *
 * Renders a looping video/GIF (QtMultimedia) or a web wallpaper (QtWebEngine)
 * as the desktop wallpaper. Because this runs inside plasmashell's wallpaper
 * layer, the desktop icons and widgets stay visible on top.
 *
 * The web part lives in a separate WebWallpaper.qml, loaded on demand, so that
 * QtWebEngine is only required when a web wallpaper is actually used.
 */
import QtQuick
import QtMultimedia

Item {
    id: root
    anchors.fill: parent

    // Config values come from contents/config/main.xml via `wallpaper.configuration`.
    readonly property url videoUrl: wallpaper.configuration.VideoUrl
    readonly property url webUrl: wallpaper.configuration.WebUrl
    readonly property bool isWeb: root.webUrl.toString().length > 0

    // Solid backdrop shown before the first frame / when nothing is set.
    Rectangle {
        anchors.fill: parent
        color: "black"
    }

    // --- Web wallpaper ------------------------------------------------------
    // Loaded by URL (not a plain import) so video-only users don't need the
    // QtWebEngine QML module installed.
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
                loops: wallpaper.configuration.Loop ? MediaPlayer.Infinite : 1
                videoOutput: videoOutput
                audioOutput: AudioOutput {
                    muted: wallpaper.configuration.Muted
                    volume: wallpaper.configuration.Volume / 100.0
                }
                onSourceChanged: if (source.toString().length > 0) play()
            }

            VideoOutput {
                id: videoOutput
                anchors.fill: parent
                fillMode: parent.fillModeIndex === 0 ? VideoOutput.Stretch
                        : parent.fillModeIndex === 1 ? VideoOutput.PreserveAspectFit
                                                     : VideoOutput.PreserveAspectCrop
            }

            Component.onCompleted: if (root.videoUrl.toString().length > 0) player.play()
        }
    }
}
