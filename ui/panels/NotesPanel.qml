import QtQuick
import ".."
import "../components"

// Quick notes.
//
// The notes are markdown files in a directory. Point that directory at an
// Obsidian vault and every note here is editable in Obsidian as well —
// which is why the panel offers to open one there rather than pretending to
// be an editor that can replace it.
Item {
    id: panel

    property var bridge: null
    property string selectedId: ""
    property string draft: ""
    property bool dirty: false

    readonly property var notes: bridge && bridge.notes ? bridge.notes : []
    readonly property var selected: {
        for (var i = 0; i < panel.notes.length; i++) {
            if (panel.notes[i].id === panel.selectedId)
                return panel.notes[i];
        }
        return null;
    }

    readonly property var directory: bridge && bridge.notesDir ? bridge.notesDir : ""

    function select(note) {
        if (panel.dirty && panel.selected)
            panel.save();
        panel.selectedId = note.id;
        panel.draft = note.body ? note.body : "";
        panel.dirty = false;
    }

    function save() {
        if (!panel.bridge || !panel.selectedId)
            return;
        panel.bridge.call("notes.set", {
            "id": panel.selectedId,
            "title": panel.selected ? panel.selected.title : "",
            "body": panel.draft
        });
        panel.dirty = false;
    }

    function newNote() {
        if (!panel.bridge)
            return;
        panel.bridge.call("notes.new", { "title": "Untitled" }, function (ok, payload) {
            if (ok && payload && payload.id) {
                panel.selectedId = payload.id;
                panel.draft = "";
                panel.dirty = false;
            }
        });
    }

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "Files"
        note: panel.notes.length > 0 ? panel.notes.length + " FILES" : ""
    }

    // The list of notes.
    ListView {
        id: list
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.rightMargin: parent.width * 0.62
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 4
        model: panel.notes

        delegate: Card {
            id: entry
            required property var modelData

            width: list.width
            height: 40
            radius: Theme.radiusSmall
            selected: panel.selectedId === entry.modelData.id
            border.width: panel.selectedId === entry.modelData.id ? 1 : 0
            border.color: Theme.accent

            Column {
                anchors.left: parent.left
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.right: parent.right
                anchors.rightMargin: Theme.spacingTight
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                Text {
                    width: parent.width
                    text: entry.modelData.title
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontSmall
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: Format.ago(entry.modelData.updated_at_unix)
                    color: Theme.textFaint
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: panel.select(entry.modelData)
            }
        }
    }

    Text {
        anchors.centerIn: list
        visible: panel.notes.length === 0
        text: "No notes yet."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontSmall
    }

    // The editor.
    Rectangle {
        id: editor
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.4
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.rightMargin: Theme.spacing
        // The footer lives below the editor, not on top of it.
        anchors.bottomMargin: Theme.spacing + 24
        color: Theme.well
        radius: Theme.radiusCard
        border.width: 1
        border.color: fieldArea.activeFocus ? Theme.accent : Theme.border

        // The note's title.
        Text {
            id: titleText
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacing
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacing
            anchors.topMargin: Theme.spacing
            text: panel.selected ? panel.selected.title : ""
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontMedium
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }

        TextInput {
            id: bodyField
            anchors.top: titleText.bottom
            anchors.topMargin: Theme.spacingTight
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacing
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacing
            anchors.bottom: parent.bottom
            anchors.bottomMargin: Theme.spacing
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontBody
            wrapMode: TextInput.Wrap
            selectByMouse: true
            selectionColor: Theme.accent
            clip: true
            text: panel.draft
            onTextChanged: {
                if (text !== panel.draft) {
                    panel.draft = text;
                    panel.dirty = true;
                }
            }
            onAccepted: panel.save()
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacing
            anchors.verticalCenter: parent.verticalCenter
            visible: bodyField.text === "" && panel.selected !== null
            text: "Write something."
            color: Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontBody
        }

        MouseArea {
            id: fieldArea
            anchors.fill: parent
            cursorShape: Qt.IBeamCursor
            onClicked: bodyField.forceActiveFocus()
        }
    }

    // The footer: the save state, the directory, and the two buttons. It is
    // a row of its own so that nothing can end up underneath the buttons.
    Row {
        id: footer
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.4
        anchors.right: newNoteButton.left
        anchors.rightMargin: Theme.spacingTight
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacingTight
        spacing: Theme.spacingTight

        Text {
            id: saveState
            anchors.verticalCenter: parent.verticalCenter
            text: panel.dirty ? "UNSAVED" : (panel.selected ? "SAVED" : "")
            color: panel.dirty ? Theme.toneWarn : Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            // Whatever the buttons and the save state have not claimed.
            width: Math.max(0, footer.width - saveState.width - footer.spacing)
            text: panel.directory
            color: Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
            elide: Text.ElideLeft
        }
    }

    IconButton {
        id: obsidianButton
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing + 32
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacingTight
        icon: "external"
        size: 28
        visible: panel.selected !== null
        tooltip: "Open in Obsidian"
        onClicked: {
            if (!panel.bridge || !panel.selected)
                return;
            panel.bridge.call("notes.obsidian", { "id": panel.selected.id }, function (ok, payload) {
                if (ok && payload && payload.uri)
                    Qt.openUrlExternally(payload.uri);
            });
        }
    }

    IconButton {
        id: newNoteButton
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacingTight
        icon: "plus"
        size: 28
        tooltip: "New note"
        onClicked: panel.newNote()
    }
}
