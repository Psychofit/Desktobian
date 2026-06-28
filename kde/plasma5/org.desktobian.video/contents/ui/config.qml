/*
 * Configuration UI for the Desktobian Video wallpaper plugin (Plasma 5 / Qt 5).
 *
 * Plasma binds each `cfg_<Key>` property to the matching entry in
 * contents/config/main.xml.
 */
import QtQuick 2.15
import QtQuick.Controls 2.15 as QtControls
import QtQuick.Layouts 1.15
import QtQuick.Dialogs 1.3 as QtDialogs
import org.kde.kirigami 2.5 as Kirigami

Kirigami.FormLayout {
    id: root

    // Bound to config/main.xml entries.
    property string cfg_VideoUrl
    property alias cfg_Muted: mutedCheck.checked
    property alias cfg_Volume: volumeSlider.value
    property alias cfg_FillMode: fillCombo.currentIndex
    property alias cfg_Loop: loopCheck.checked
    property alias cfg_MouseInteraction: interactionCheck.checked

    // Defaults (used by Plasma's "reset to defaults").
    property string cfg_VideoUrlDefault: ""
    property bool cfg_MutedDefault: true
    property int cfg_VolumeDefault: 100
    property int cfg_FillModeDefault: 2
    property bool cfg_LoopDefault: true
    property bool cfg_MouseInteractionDefault: false

    RowLayout {
        Kirigami.FormData.label: i18n("Video:")

        QtControls.TextField {
            id: pathField
            Layout.fillWidth: true
            Layout.minimumWidth: Kirigami.Units.gridUnit * 18
            readOnly: true
            placeholderText: i18n("No video selected")
            text: root.cfg_VideoUrl
        }
        QtControls.Button {
            text: i18n("Browse…")
            icon.name: "document-open"
            onClicked: fileDialog.open()
        }
    }

    QtControls.CheckBox {
        id: mutedCheck
        Kirigami.FormData.label: i18n("Audio:")
        text: i18n("Muted")
    }

    QtControls.Slider {
        id: volumeSlider
        Kirigami.FormData.label: i18n("Volume:")
        from: 0
        to: 100
        stepSize: 1
        enabled: !mutedCheck.checked
        Layout.minimumWidth: Kirigami.Units.gridUnit * 12
    }

    QtControls.ComboBox {
        id: fillCombo
        Kirigami.FormData.label: i18n("Fill mode:")
        model: [
            i18n("Stretch (ignore aspect ratio)"),
            i18n("Fit (letterbox)"),
            i18n("Crop (fill screen)")
        ]
    }

    QtControls.CheckBox {
        id: loopCheck
        Kirigami.FormData.label: i18n("Playback:")
        text: i18n("Loop forever")
    }

    QtControls.CheckBox {
        id: interactionCheck
        Kirigami.FormData.label: i18n("Web interaction:")
        text: i18n("Forward mouse to web wallpapers (uses right-click)")
    }

    QtDialogs.FileDialog {
        id: fileDialog
        title: i18n("Choose a video")
        nameFilters: [
            i18n("Video files (*.mp4 *.mkv *.webm *.mov *.avi *.m4v *.gif)"),
            i18n("All files (*)")
        ]
        onAccepted: root.cfg_VideoUrl = fileDialog.fileUrl
    }
}
