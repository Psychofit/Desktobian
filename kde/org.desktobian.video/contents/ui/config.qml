/*
 * Configuration UI for the Desktobian Video wallpaper plugin.
 *
 * Plasma binds each `cfg_<Key>` property to the matching entry in
 * contents/config/main.xml.
 */
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Dialogs as QtDialogs
import org.kde.kcmutils as KCM
import org.kde.kirigami as Kirigami

KCM.SimpleKCM {
    id: root

    // Bound to config/main.xml entries.
    property string cfg_VideoUrl
    property alias cfg_Muted: mutedCheck.checked
    property alias cfg_Volume: volumeSlider.value
    property alias cfg_FillMode: fillCombo.currentIndex
    property alias cfg_Loop: loopCheck.checked

    // Defaults (used by Plasma's "reset to defaults").
    property string cfg_VideoUrlDefault: ""
    property bool cfg_MutedDefault: true
    property int cfg_VolumeDefault: 100
    property int cfg_FillModeDefault: 2
    property bool cfg_LoopDefault: true

    Kirigami.FormLayout {
        RowLayout {
            Kirigami.FormData.label: i18n("Video:")

            QQC2.TextField {
                id: pathField
                Layout.fillWidth: true
                Layout.minimumWidth: Kirigami.Units.gridUnit * 18
                readOnly: true
                placeholderText: i18n("No video selected")
                text: root.cfg_VideoUrl
            }
            QQC2.Button {
                text: i18n("Browse…")
                icon.name: "document-open"
                onClicked: fileDialog.open()
            }
        }

        QQC2.CheckBox {
            id: mutedCheck
            Kirigami.FormData.label: i18n("Audio:")
            text: i18n("Muted")
        }

        QQC2.Slider {
            id: volumeSlider
            Kirigami.FormData.label: i18n("Volume:")
            from: 0
            to: 100
            stepSize: 1
            enabled: !mutedCheck.checked
            Layout.minimumWidth: Kirigami.Units.gridUnit * 12
        }

        QQC2.ComboBox {
            id: fillCombo
            Kirigami.FormData.label: i18n("Fill mode:")
            model: [
                i18n("Stretch (ignore aspect ratio)"),
                i18n("Fit (letterbox)"),
                i18n("Crop (fill screen)")
            ]
        }

        QQC2.CheckBox {
            id: loopCheck
            Kirigami.FormData.label: i18n("Playback:")
            text: i18n("Loop forever")
        }
    }

    QtDialogs.FileDialog {
        id: fileDialog
        title: i18n("Choose a video")
        nameFilters: [
            i18n("Video files (*.mp4 *.mkv *.webm *.mov *.avi *.m4v *.gif)"),
            i18n("All files (*)")
        ]
        onAccepted: root.cfg_VideoUrl = selectedFile
    }
}
