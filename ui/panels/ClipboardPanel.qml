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
    TextField {
        id: searchBox
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: implicitHeight
        variant: "outlined"
        icon: "search"
        placeholder: "Search the history"
        text: panel.filter
        onTextChanged: panel.filter = text
        onEscapePressed: {
            panel.filter = "";
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

        Button {
            variant: "outlined"
            text: "Capture"
            icon: "copy"
            onClicked: panel.bridge ? panel.bridge.call("clipboard.capture", {}) : null
        }

        Button {
            variant: "text"
            text: "Clear"
            visible: panel.entries.length > 0
            onClicked: panel.bridge ? panel.bridge.call("clipboard.clear", {}) : null
        }
    }

    Text {
        anchors.centerIn: parent
        visible: panel.entries.length === 0
        text: "The clipboard history is empty."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    Text {
        anchors.top: actions.bottom
        anchors.topMargin: Theme.spacing
        anchors.horizontalCenter: parent.horizontalCenter
        visible: panel.entries.length > 0 && panel.visibleEntries.length === 0
        text: "Nothing matches."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
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
        spacing: 6
        model: panel.visibleEntries

        delegate: Card {
            id: entry
            required property var modelData

            width: list.width
            height: 34
            radius: Theme.radiusMedium

            Row {
                id: clipRow
                anchors.fill: parent
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.rightMargin: Theme.spacingTight
                spacing: Theme.spacingTight

                Chip {
                    id: kindChip
                    anchors.verticalCenter: parent.verticalCenter
                    text: entry.modelData.kind ? entry.modelData.kind.toUpperCase() : "TEXT"
                    tone: entry.modelData.kind === "url"
                        ? Theme.toneInfo
                        : (entry.modelData.kind === "code" ? Theme.toneGood : Theme.onSurfaceVariant)
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    // Everything the row does not need for itself.
                    width: Math.max(0, clipRow.width - kindChip.width
                        - ageText.width - pinButton.width - copyButton.width
                        - trashButton.width - clipRow.spacing * 5)
                    text: entry.modelData.text
                    color: Theme.onSurface
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.type.bodySmall.size
                    font.weight: Theme.type.bodySmall.weight
                    font.letterSpacing: Theme.type.bodySmall.tracking
                    elide: Text.ElideRight
                }

                Text {
                    id: ageText
                    anchors.verticalCenter: parent.verticalCenter
                    text: Format.ago(entry.modelData.created_at_unix)
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                }

                IconButton {
                    id: pinButton
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
                    id: copyButton
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "copy"
                    size: 22
                    tooltip: "Copy back"
                    onClicked: panel.bridge
                        ? panel.bridge.call("clipboard.paste", { "id": entry.modelData.id })
                        : null
                }

                IconButton {
                    id: trashButton
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
