import QtQuick
import ".."
import "../components"

// The wallpaper switcher, and the theme that goes with it.
//
// The catalog is a directory of images the daemon owns; the engine is a
// separate process that draws them. A wallpaper can be applied by clicking
// it, the rotation can be left running, and the directory can be pointed
// somewhere else.
Item {
    id: panel

    property var bridge: null
    property string dirDraft: ""
    property bool dirEditing: false

    readonly property var payload: bridge && bridge.wallpapers ? bridge.wallpapers : null
    readonly property var items: payload && payload.items ? payload.items : []
    readonly property var current: payload && payload.current ? payload.current : null
    readonly property bool engineOk: payload && payload.engine_ok === true
    readonly property bool loopOn: payload && payload.loop_enabled === true

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "Wallpapers"
        note: panel.items.length > 0 ? panel.items.length + " IMAGES" : ""
    }

    // The engine and the rotation.
    Card {
        id: controls
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 46

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight + 2

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "image"
                width: 15
                height: 15
                color: panel.engineOk ? Theme.toneInfo : Theme.toneWarn
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 190
                spacing: 1

                Text {
                    width: parent.width
                    text: panel.payload && panel.payload.engine
                        ? "Engine · " + panel.payload.engine
                        : "Engine · none"
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontSmall
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: panel.engineOk
                        ? "ready"
                        : "not answering — the catalog still works"
                    color: panel.engineOk ? Theme.textDim : Theme.toneWarn
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                    elide: Text.ElideRight
                }
            }

            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                icon: "refresh"
                size: 28
                tooltip: "Pick a random wallpaper"
                onClicked: panel.bridge ? panel.bridge.call("wallpaper.next", {}) : null
            }

            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                icon: "repeat"
                size: 28
                active: panel.loopOn
                tooltip: panel.loopOn ? "Stop the rotation" : "Rotate every minute"
                onClicked: panel.bridge
                    ? panel.bridge.call("wallpaper.loop", {
                        "enabled": !panel.loopOn,
                        "interval_secs": 60
                    })
                    : null
            }
        }
    }

    // The directory.
    Card {
        id: dirCard
        anchors.top: controls.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 36

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "folder"
                width: 14
                height: 14
                color: Theme.textFaint
            }

            TextInput {
                id: dirField
                anchors.verticalCenter: parent.verticalCenter
                width: Math.max(0, parent.width - 110)
                color: Theme.text
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontTiny
                selectByMouse: true
                selectionColor: Theme.accent
                clip: true
                text: panel.dirEditing
                    ? panel.dirDraft
                    : (panel.payload && panel.payload.dir ? panel.payload.dir : "")
                onTextChanged: {
                    if (panel.dirEditing)
                        panel.dirDraft = text;
                }
                onAccepted: panel.applyDirectory()
                Keys.onEscapePressed: {
                    panel.dirEditing = false;
                    panel.dirDraft = "";
                    focus = false;
                }
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: !panel.dirEditing
                text: "change"
                color: changeArea.containsMouse ? Theme.text : Theme.textFaint
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontTiny
                font.weight: Font.Medium

                MouseArea {
                    id: changeArea
                    anchors.fill: parent
                    anchors.margins: -6
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        panel.dirEditing = true;
                        panel.dirDraft = dirField.text;
                        dirField.forceActiveFocus();
                    }
                }
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                visible: panel.dirEditing
                width: applyLabel.implicitWidth + 16
                height: 24
                radius: Theme.radiusSmall
                color: Theme.accentDim
                border.width: 1
                border.color: Theme.accent

                Text {
                    id: applyLabel
                    anchors.centerIn: parent
                    text: "Apply"
                    color: Theme.accent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                    font.weight: Font.DemiBold
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: panel.applyDirectory()
                }
            }
        }
    }

    function applyDirectory() {
        if (!panel.bridge || !panel.dirEditing)
            return;
        var dir = panel.dirDraft.trim();
        if (dir === "")
            return;
        panel.bridge.call("wallpaper.setdir", { "dir": dir });
        panel.dirEditing = false;
    }

    // The grid.
    Grid {
        id: grid
        anchors.top: dirCard.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing
        columns: 4
        columnSpacing: Theme.spacingTight
        rowSpacing: Theme.spacingTight
        clip: true

        Repeater {
            model: panel.items

            Item {
                required property var modelData

                width: (grid.width - Theme.spacingTight * 3) / 4
                height: width * 0.62

                Rectangle {
                    id: frame
                    anchors.fill: parent
                    radius: Theme.radiusSmall
                    color: Theme.well
                    border.width: panel.current === modelData.path ? 2 : 0
                    border.color: Theme.accent
                    clip: true

                    Image {
                        anchors.fill: parent
                        anchors.margins: 1
                        // The thumbnail is the file itself, decoded small.
                        // The catalog holds paths, not copies.
                        source: panel.visible && modelData.path ? "file://" + modelData.path : ""
                        sourceSize: Qt.size(240, 150)
                        asynchronous: true
                        fillMode: Image.PreserveAspectCrop
                        smooth: true
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: parent.radius
                        color: "transparent"
                        border.width: 1
                        border.color: Theme.border
                    }
                }

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.radiusSmall
                    color: "transparent"
                    visible: tileArea.containsMouse

                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 1
                        radius: Theme.radiusSmall
                        color: Qt.rgba(0, 0, 0, 0.45)
                    }
                }

                Text {
                    anchors.bottom: parent.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.margins: 5
                    visible: tileArea.containsMouse
                    text: modelData.name
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                MouseArea {
                    id: tileArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: panel.bridge
                        ? panel.bridge.call("wallpaper.set", { "path": modelData.path })
                        : null
                }
            }
        }
    }

    Text {
        anchors.centerIn: grid
        visible: panel.items.length === 0
        text: "No images in that directory."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontBody
    }
}
