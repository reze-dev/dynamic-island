import QtQuick
import ".."
import "../components"

// Todos and the pomodoro.
//
// The list on the left, the clock on the right. A todo may carry a deadline;
// the deadline can be written inline, as "standup 15m" or "ship it 2h", and
// the panel works out the rest.
Item {
    id: panel

    property var bridge: null
    property string draft: ""

    readonly property var todos: bridge && bridge.todos ? bridge.todos : []
    readonly property var timer: bridge && bridge.timer ? bridge.timer : null
    readonly property bool timerRunning: timer && timer.running === true
    readonly property bool timerIdle: !timer || timer.phase === "idle"

    /// The todos that are still to do, and the ones that are not.
    readonly property var pending: {
        var list = [];
        for (var i = 0; i < panel.todos.length; i++) {
            if (!panel.todos[i].done)
                list.push(panel.todos[i]);
        }
        return list;
    }

    readonly property var finished: {
        var list = [];
        for (var i = 0; i < panel.todos.length; i++) {
            if (panel.todos[i].done)
                list.push(panel.todos[i]);
        }
        return list;
    }

    /// Pull a deadline out of the end of a line: "15m", "2h", "1d".
    function splitDeadline(text) {
        var match = /\s+(\d+)\s*([mhd])$/.exec(text);
        if (!match)
            return { text: text, minutes: 0 };
        var amount = parseInt(match[1]);
        var unit = match[2];
        var minutes = unit === "h" ? amount * 60 : (unit === "d" ? amount * 60 * 24 : amount);
        return {
            text: text.slice(0, match.index),
            minutes: minutes
        };
    }

    function addTodo() {
        var raw = panel.draft.trim();
        if (raw === "")
            return;
        var parts = panel.splitDeadline(raw);
        var args = { "text": parts.text === "" ? raw : parts.text };
        if (parts.minutes > 0)
            args["due_in_minutes"] = parts.minutes;
        if (panel.bridge)
            panel.bridge.call("todos.add", args);
        panel.draft = "";
    }

    // ---- the list ----

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.rightMargin: parent.width * 0.42
        anchors.margins: Theme.spacing
        title: "Open tasks"
        note: panel.pending.length > 0 ? panel.pending.length + " OPEN" : ""
    }

    ListView {
        id: list
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.rightMargin: parent.width * 0.42
        anchors.bottom: entryField.top
        anchors.bottomMargin: Theme.spacingTight
        anchors.leftMargin: Theme.spacing
        clip: true
        spacing: 4
        model: panel.pending

        delegate: Card {
            id: entry
            required property var modelData

            width: list.width
            height: 38
            radius: Theme.radiusSmall

            Row {
                id: todoRow
                anchors.fill: parent
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.rightMargin: Theme.spacingTight
                spacing: Theme.spacingTight

                // The tick box.
                Rectangle {
                    id: tickBox
                    anchors.verticalCenter: parent.verticalCenter
                    width: 17
                    height: 17
                    radius: 5
                    color: tickArea.containsMouse ? Theme.accentDim : "transparent"
                    border.width: 1
                    border.color: tickArea.containsMouse ? Theme.accent : Theme.borderStrong

                    Icon {
                        anchors.centerIn: parent
                        visible: tickArea.containsMouse
                        name: "check"
                        width: 10
                        height: 10
                        color: Theme.accent
                    }

                    MouseArea {
                        id: tickArea
                        anchors.fill: parent
                        anchors.margins: -4
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.bridge
                            ? panel.bridge.call("todos.toggle", { "id": entry.modelData.id })
                            : null
                    }
                }

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    // Whatever is left once the tick box and the button have
                    // had their share. A fixed number here would leave the
                    // button floating in the middle of the row.
                    width: Math.max(0, todoRow.width - tickBox.width
                        - removeButton.width - todoRow.spacing * 2)
                    spacing: 1

                    Text {
                        width: parent.width
                        text: entry.modelData.text
                        color: Theme.text
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.fontBody
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        visible: entry.modelData.due_unix !== undefined
                            && entry.modelData.due_unix !== null
                        text: "due " + Format.until(entry.modelData.due_unix)
                            + " · " + Format.timeOfDay(entry.modelData.due_unix)
                        color: entry.modelData.due_unix * 1 <= Math.floor(Date.now() / 1000)
                            ? Theme.toneWarn
                            : Theme.textFaint
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.fontTiny
                        elide: Text.ElideRight
                    }
                }

                IconButton {
                    id: removeButton
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "trash"
                    size: 22
                    tooltip: "Remove"
                    onClicked: panel.bridge
                        ? panel.bridge.call("todos.remove", { "id": entry.modelData.id })
                        : null
                }
            }
        }
    }

    Text {
        anchors.centerIn: list
        visible: panel.pending.length === 0
        text: "Nothing on the list."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontBody
    }

    // The finished ones, quietly underneath.
    Text {
        id: doneLabel
        anchors.left: parent.left
        anchors.leftMargin: Theme.spacing
        anchors.bottom: entryField.top
        anchors.bottomMargin: Theme.spacingTight
        visible: panel.finished.length > 0
        text: panel.finished.length + " done"
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontTiny
    }

    // The entry field.
    Rectangle {
        id: entryField
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.rightMargin: parent.width * 0.42
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing
        anchors.leftMargin: Theme.spacing
        height: 34
        radius: Theme.radiusSmall
        color: Theme.well
        border.width: 1
        border.color: fieldArea.activeFocus ? Theme.accent : Theme.border

        Text {
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacingTight + 2
            anchors.verticalCenter: parent.verticalCenter
            visible: todoField.text === ""
            text: "Add a todo — \"standup 15m\""
            color: Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontSmall
        }

        TextInput {
            id: todoField
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacingTight + 2
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacingTight
            anchors.verticalCenter: parent.verticalCenter
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontSmall
            selectByMouse: true
            selectionColor: Theme.accent
            clip: true
            text: panel.draft
            onTextChanged: panel.draft = text
            onAccepted: panel.addTodo()
            Keys.onEscapePressed: {
                panel.draft = "";
                focus = false;
            }
        }

        MouseArea {
            id: fieldArea
            anchors.fill: parent
            cursorShape: Qt.IBeamCursor
            onClicked: todoField.forceActiveFocus()
        }
    }

    // ---- the pomodoro ----

    Rectangle {
        id: timerCard
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.leftMargin: parent.width * 0.6
        anchors.rightMargin: Theme.spacing
        anchors.topMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        color: Theme.card
        radius: Theme.radiusCard
        border.width: 1
        border.color: Theme.border

        Column {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "POMODORO"
                color: Theme.textFaint
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontTiny
                font.weight: Font.DemiBold
                font.letterSpacing: 1.4
            }

            ProgressRing {
                id: ring
                anchors.horizontalCenter: parent.horizontalCenter
                width: 118
                height: 118
                progress: panel.timer ? (panel.timer.progress || 0) : 0
                tone: panel.timer && panel.timer.phase === "short-break" ? Theme.toneGood
                    : (panel.timer && panel.timer.phase === "long-break" ? Theme.toneInfo : Theme.toneWarn)

                caption: panel.timer && panel.timer.remaining_ms !== null
                    && panel.timer.remaining_ms !== undefined
                    ? Format.duration(panel.timer.remaining_ms)
                    : "25:00"
                subcaption: panel.timer && panel.timer.phase ? panel.timer.phase : "idle"
            }

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: panel.timer && panel.timer.rounds_completed !== undefined
                    ? panel.timer.rounds_completed + " rounds done"
                    : ""
                color: Theme.textFaint
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontTiny
            }

            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.spacingTight

                IconButton {
                    icon: "play"
                    size: 32
                    active: panel.timerRunning
                    tooltip: panel.timerRunning ? "Pause" : "Start"
                    onClicked: {
                        if (!panel.bridge)
                            return;
                        if (panel.timerRunning)
                            panel.bridge.call("timer.pause", {});
                        else if (panel.timerIdle)
                            panel.bridge.call("timer.start", {});
                        else
                            panel.bridge.call("timer.resume", {});
                    }
                }

                IconButton {
                    icon: "refresh"
                    size: 32
                    tooltip: "Restart"
                    onClicked: panel.bridge ? panel.bridge.call("timer.start", {}) : null
                }

                IconButton {
                    icon: "close"
                    size: 32
                    tooltip: "Stop"
                    onClicked: panel.bridge ? panel.bridge.call("timer.stop", {}) : null
                }
            }
        }
    }
}
