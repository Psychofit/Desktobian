/*
 * Configuration UI for the Desktobian Video wallpaper plugin.
 *
 * Plasma binds each `cfg_<Key>` property to the matching entry in
 * contents/config/main.xml.
 *
 * For web wallpapers it also renders a property editor built from the
 * wallpaper's project.json `general.properties` (colours, sliders, combos,
 * toggles, …), persisting the user's choices as a sparse JSON override map in
 * the WebProperties config entry. The parsing/merging lives in we-properties.js
 * so it can be unit-tested.
 */
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Dialogs as QtDialogs
import org.kde.kcmutils as KCM
import org.kde.kirigami as Kirigami
import "we-properties.js" as WP

KCM.SimpleKCM {
    id: root

    // Bound to config/main.xml entries.
    property string cfg_VideoUrl
    property string cfg_WebUrl
    property string cfg_WebProperties
    property alias cfg_Muted: mutedCheck.checked
    property alias cfg_Volume: volumeSlider.value
    property alias cfg_FillMode: fillCombo.currentIndex
    property alias cfg_Loop: loopCheck.checked
    property alias cfg_MouseInteraction: interactionCheck.checked

    // Defaults (used by Plasma's "reset to defaults").
    property string cfg_VideoUrlDefault: ""
    property string cfg_WebUrlDefault: ""
    property string cfg_WebPropertiesDefault: ""
    property bool cfg_MutedDefault: true
    property int cfg_VolumeDefault: 100
    property int cfg_FillModeDefault: 2
    property bool cfg_LoopDefault: true
    property bool cfg_MouseInteractionDefault: false

    // Parsed property model for the current web wallpaper (empty for videos).
    property var webPropModel: []
    // The colour property currently being edited via the shared ColorDialog.
    property var colorTarget: null

    Component.onCompleted: loadWebProperties()
    onCfg_WebUrlChanged: loadWebProperties()

    // Read the web wallpaper's project.json (sibling of its index.html) and turn
    // its general.properties into the editable model.
    function loadWebProperties() {
        webPropModel = [];
        var w = cfg_WebUrl ? cfg_WebUrl.toString() : "";
        if (w.length === 0) {
            return;
        }
        var dir = w.substring(0, w.lastIndexOf("/") + 1);
        var url = dir + "project.json";
        var xhr = new XMLHttpRequest();
        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                root.webPropModel = WP.parseProperties(xhr.responseText || "");
            }
        };
        try {
            xhr.open("GET", url);
            xhr.send();
        } catch (e) {
            // Leave the model empty; the editor section just won't appear.
        }
    }

    // The effective current value for a property (override, else default).
    function currentValue(name, def) {
        return WP.valueFor(WP.parseOverrides(cfg_WebProperties), name, def);
    }

    // Persist a changed value (clearing it when it matches the default).
    function applyValue(name, value, def) {
        cfg_WebProperties = WP.withOverride(cfg_WebProperties, name, value, def);
    }

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

        QQC2.CheckBox {
            id: interactionCheck
            Kirigami.FormData.label: i18n("Web interaction:")
            text: i18n("Forward mouse to web wallpapers (uses right-click)")
        }

        // --- Web wallpaper property editor ---------------------------------
        Item {
            Kirigami.FormData.label: i18n("Wallpaper properties")
            Kirigami.FormData.isSection: true
            visible: root.webPropModel.length > 0
        }

        Repeater {
            model: root.webPropModel

            delegate: Loader {
                property var pm: modelData
                Kirigami.FormData.label: (modelData.label || modelData.name) + ":"
                sourceComponent: modelData.type === "bool" ? boolComp
                               : modelData.type === "slider" ? sliderComp
                               : modelData.type === "combo" ? comboComp
                               : modelData.type === "color" ? colorComp
                               : textComp
            }
        }
    }

    // --- Per-type property controls ---------------------------------------
    // Each reads its initial value in Component.onCompleted (so user edits via
    // the on-change signals don't fight a binding) and writes back through
    // root.applyValue. `parent.pm` is the model entry set on the Loader.

    Component {
        id: boolComp
        QQC2.CheckBox {
            readonly property var pm: parent.pm
            Component.onCompleted: checked = (root.currentValue(pm.name, pm.def) === true)
            onToggled: root.applyValue(pm.name, checked, pm.def)
        }
    }

    Component {
        id: sliderComp
        RowLayout {
            readonly property var pm: parent.pm
            QQC2.Slider {
                id: sld
                Layout.minimumWidth: Kirigami.Units.gridUnit * 12
                from: pm.min
                to: pm.max
                stepSize: pm.step > 0 ? pm.step : 0
                Component.onCompleted: value = root.currentValue(pm.name, pm.def)
                onMoved: root.applyValue(pm.name, value, pm.def)
            }
            QQC2.Label {
                text: pm.step >= 1 ? Math.round(sld.value) : sld.value.toFixed(2)
            }
        }
    }

    Component {
        id: comboComp
        QQC2.ComboBox {
            readonly property var pm: parent.pm
            textRole: "label"
            model: pm.options
            Component.onCompleted: {
                var cur = root.currentValue(pm.name, pm.def);
                for (var i = 0; i < pm.options.length; i++) {
                    if (pm.options[i].value === cur) {
                        currentIndex = i;
                        break;
                    }
                }
            }
            onActivated: root.applyValue(pm.name, pm.options[currentIndex].value, pm.def)
        }
    }

    Component {
        id: colorComp
        RowLayout {
            readonly property var pm: parent.pm
            Rectangle {
                Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                Layout.preferredHeight: Kirigami.Units.gridUnit * 1.5
                radius: 3
                border.width: 1
                border.color: Kirigami.Theme.textColor
                color: {
                    var c = WP.colorToRgb(root.currentValue(pm.name, pm.def));
                    return Qt.rgba(c.r / 255, c.g / 255, c.b / 255, 1);
                }
            }
            QQC2.Button {
                text: i18n("Change…")
                onClicked: {
                    root.colorTarget = pm;
                    var c = WP.colorToRgb(root.currentValue(pm.name, pm.def));
                    colorDialog.selectedColor = Qt.rgba(c.r / 255, c.g / 255, c.b / 255, 1);
                    colorDialog.open();
                }
            }
        }
    }

    Component {
        id: textComp
        QQC2.TextField {
            readonly property var pm: parent.pm
            Layout.minimumWidth: Kirigami.Units.gridUnit * 12
            Component.onCompleted: text = String(root.currentValue(pm.name, pm.def))
            onEditingFinished: root.applyValue(pm.name, text, pm.def)
        }
    }

    QtDialogs.ColorDialog {
        id: colorDialog
        onAccepted: {
            if (!root.colorTarget) {
                return;
            }
            var c = colorDialog.selectedColor;
            var s = WP.rgbToColorString(
                Math.round(c.r * 255), Math.round(c.g * 255), Math.round(c.b * 255));
            root.applyValue(root.colorTarget.name, s, root.colorTarget.def);
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
