import QtQuick
import ".."
import "../components"

// The clipboard history.
//
// Entries are classified when they arrive, so the kind is a label rather
// than a guess at read time. Pinned entries are kept when the history is
// trimmed.
Item {
    id: panel

    property var bridge: null
    property string filter: ""

    readonly property var entries: bridge && bridge.clipboard ? bridge.clipboard : []

    readonly property var visibleEntries: {
        var needle = panel.filter.trim().toLowerCase();
        if (needle === "")
            return panel.entries;
        return panel.entries.filter(function (entry) {
            return entry.text.toLowerCase().indexOf(needle) !== -1;
        });
    }

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "History"
        note: panel.entries.length > 0 ? panel.entries.length + " ITEMS" : ""
    }

    // The search box.
    Rectangle {
        id: searchBox
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 32
        radius: Theme.radiusSmall
        color: Theme.well
        border.width: 1
        border.color: searchArea.activeFocus ? Theme.accent : Theme.border

        Icon {
            id: searchIcon
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacingTight + 2
            anchors.verticalCenter: parent.verticalCenter
            name: "search"
            width: 13
            height: 13
            color: Theme.textFaint
        }

        TextInput {
            id: searchField
            anchors.left: searchIcon.right
            anchors.leftMargin: Theme.spacingTight
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacingTight
            anchors.verticalCenter: parent.verticalCenter
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontSmall
            selectByMouse: true
            selectionColor: Theme.accent
            clip: true
            text: panel.filter
            onTextChanged: panel.filter = text
            Keys.onEscapePressed: {
                panel.filter = "";
                focus = false;
            }
        }

        Text {
            anchors.left: searchIcon.right
            anchors.leftMargin: Theme.spacingTight
            anchors.verticalCenter: parent.verticalCenter
            visible: searchField.text === ""
            text: "Search the history"
            color: Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontSmall
        }

        MouseArea {
            id: searchArea
            anchors.fill: parent
            cursorShape: Qt.IBeamCursor
            onClicked: searchField.forceActiveFocus()
        }
    }

    // The actions.
    Row {
        id: actions
        anchors.top: searchBox.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        spacing: Theme.spacingTight

        Rectangle {
            width: captureLabel.implicitWidth + 20
            height: 26
            radius: Theme.radiusSmall
            color: captureArea.containsMouse ? Theme.cardHover : Theme.card
            border.width: 1
            border.color: Theme.border

            Row {
                anchors.centerIn: parent
                spacing: 5

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "copy"
                    width: 12
                    height: 12
                    color: Theme.textDim
                }

                Text {
                    id: captureLabel
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Capture"
                    color: Theme.textDim
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                    font.weight: Font.Medium
                }
            }

            MouseArea {
                id: captureArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: panel.bridge ? panel.bridge.call("clipboard.capture", {}) : null
            }
        }

        Rectangle {
            width: clearLabel.implicitWidth + 20
            height: 26
            radius: Theme.radiusSmall
            color: clearArea.containsMouse ? Theme.cardHover : Theme.card
            border.width: 1
            border.color: Theme.border
            visible: panel.entries.length > 0

            Text {
                id: clearLabel
                anchors.centerIn: parent
                text: "Clear"
                color: Theme.textDim
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontTiny
                font.weight: Font.Medium
            }

            MouseArea {
                id: clearArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: panel.bridge ? panel.bridge.call("clipboard.clear", {}) : null
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: panel.entries.length === 0
        text: "The clipboard history is empty."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontBody
    }

    Text {
        anchors.top: actions.bottom
        anchors.topMargin: Theme.spacing
        anchors.horizontalCenter: parent.horizontalCenter
        visible: panel.entries.length > 0 && panel.visibleEntries.length === 0
        text: "Nothing matches."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontBody
    }

    ListView {
        id: list
        anchors.top: actions.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 5
        model: panel.visibleEntries

        delegate: Card {
            id: entry
            required property var modelData

            width: list.width
            height: 34
            radius: Theme.radiusSmall

            Row {
                anchors.fill: parent
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.rightMargin: Theme.spacingTight
                spacing: Theme.spacingTight

                Chip {
                    anchors.verticalCenter: parent.verticalCenter
                    text: entry.modelData.kind ? entry.modelData.kind.toUpperCase() : "TEXT"
                    tone: entry.modelData.kind === "url"
                        ? Theme.toneInfo
                        : (entry.modelData.kind === "code" ? Theme.toneGood : Theme.textDim)
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 190)
                    text: entry.modelData.text
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontSmall
                    elide: Text.ElideRight
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Format.ago(entry.modelData.created_at_unix)
                    color: Theme.textFaint
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                }

                IconButton {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "pin"
                    size: 22
                    active: entry.modelData.pinned === true
                    tooltip: entry.modelData.pinned ? "Unpin" : "Pin"
                    onClicked: panel.bridge
                        ? panel.bridge.call("clipboard.pin", {
                            "id": entry.modelData.id,
                            "pinned": !(entry.modelData.pinned === true)
                        })
                        : null
                }

                IconButton {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "copy"
                    size: 22
                    tooltip: "Copy back"
                    onClicked: panel.bridge
                        ? panel.bridge.call("clipboard.paste", { "id": entry.modelData.id })
                        : null
                }

                IconButton {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "trash"
                    size: 22
                    tooltip: "Remove"
                    onClicked: panel.bridge
                        ? panel.bridge.call("clipboard.remove", { "id": entry.modelData.id })
                        : null
                }
            }
        }
    }
}
