/*
 * Desktobian Video — native Plasma 5 wallpaper plugin (Qt 5 / QtMultimedia 5).
 *
 * Renders a looping video/GIF as the desktop wallpaper inside plasmashell's
 * wallpaper layer, so desktop icons and widgets remain visible on top.
 */
import QtQuick 2.15
import QtMultimedia 5.15

Item {
    id: root
    anchors.fill: parent

    // Config values from contents/config/main.xml via `wallpaper.configuration`.
    readonly property url videoUrl: wallpaper.configuration.VideoUrl
    readonly property bool muted: wallpaper.configuration.Muted
    readonly property int volume: wallpaper.configuration.Volume
    readonly property int fillModeIndex: wallpaper.configuration.FillMode
    readonly property bool loop: wallpaper.configuration.Loop

    // Backdrop shown before the first frame / when no video is set.
    Rectangle {
        anchors.fill: parent
        color: "black"
    }

    MediaPlayer {
        id: player
        source: root.videoUrl
        autoPlay: true
        loops: root.loop ? MediaPlayer.Infinite : 1
        muted: root.muted
        volume: root.volume / 100.0
    }

    VideoOutput {
        anchors.fill: parent
        source: player
        fillMode: root.fillModeIndex === 0 ? VideoOutput.Stretch
                : root.fillModeIndex === 1 ? VideoOutput.PreserveAspectFit
                                           : VideoOutput.PreserveAspectCrop
    }

    // Re-apply when the chosen video changes.
    onVideoUrlChanged: {
        player.stop()
        player.source = root.videoUrl
        if (root.videoUrl.toString().length > 0) {
            player.play()
        }
    }
}
