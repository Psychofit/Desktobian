/*
 * Desktobian Video — a native Plasma 6 wallpaper plugin.
 *
 * Renders a looping video/GIF as the desktop wallpaper using QtMultimedia.
 * Because this runs inside plasmashell's wallpaper layer, the desktop icons
 * and widgets stay visible on top of the video (unlike an external window).
 */
import QtQuick
import QtMultimedia

Item {
    id: root
    anchors.fill: parent

    // Config values come from contents/config/main.xml via `wallpaper.configuration`.
    readonly property url videoUrl: wallpaper.configuration.VideoUrl
    readonly property bool muted: wallpaper.configuration.Muted
    readonly property int volume: wallpaper.configuration.Volume
    readonly property int fillModeIndex: wallpaper.configuration.FillMode
    readonly property bool loop: wallpaper.configuration.Loop

    // Solid backdrop shown before the first frame / when no video is set.
    Rectangle {
        anchors.fill: parent
        color: "black"
    }

    MediaPlayer {
        id: player
        source: root.videoUrl
        loops: root.loop ? MediaPlayer.Infinite : 1
        videoOutput: videoOutput
        audioOutput: AudioOutput {
            muted: root.muted
            volume: root.volume / 100.0
        }
        // (Re)start whenever the source changes to a real file.
        onSourceChanged: if (source.toString().length > 0) play()
    }

    VideoOutput {
        id: videoOutput
        anchors.fill: parent
        fillMode: root.fillModeIndex === 0 ? VideoOutput.Stretch
                : root.fillModeIndex === 1 ? VideoOutput.PreserveAspectFit
                                           : VideoOutput.PreserveAspectCrop
    }

    Component.onCompleted: if (root.videoUrl.toString().length > 0) player.play()

    // Re-apply live config changes (e.g. after picking a new video).
    onVideoUrlChanged: if (root.videoUrl.toString().length > 0) player.play()
}
