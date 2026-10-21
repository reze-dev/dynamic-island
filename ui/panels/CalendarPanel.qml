import QtQuick
import ".."
import "../components"

// The calendar.
//
// An agenda, not a month grid: the next few hours, laid out by the hour, so
// it answers "what is coming" rather than "what day is it". The month grid
// is a different job and belongs in a different place.
Item {
    id: panel

    property var bridge: null

    readonly property var payload: bridge && bridge.calendar ? bridge.calendar : null
    readonly property var events: payload && payload.events ? payload.events : []
    readonly property var currentEvent: payload && payload.current ? payload.current : null

    /// The next fourteen hours, each with the events that fall in it.
    readonly property var hours: {
        var now = Math.floor(Date.now() / 1000);
        var anchor = new Date(now * 1000);
        anchor.setMinutes(0, 0, 0);
        var list = [];
        for (var i = 0; i < 14; i++) {
            var start = Math.floor(anchor.getTime() / 1000) + i * 3600;
            var inHour = [];
            for (var j = 0; j < panel.events.length; j++) {
                var event = panel.events[j];
                if (event.start_unix >= start && event.start_unix < start + 3600)
                    inHour.push(event);
            }
            list.push({
                start: start,
                events: inHour,
                isNow: i === 0,
                isPast: start + 3600 <= now
            });
        }
        return list;
    }

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: panel.currentEvent ? "In progress" : "Up next"
        note: panel.events.length > 0 ? panel.events.length + " EVENTS" : ""
    }

    // The event that is happening right now, if any.
    Card {
        id: currentCard
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 52
        visible: panel.currentEvent !== null
        border.width: 1
        border.color: Qt.rgba(Theme.toneGood.r, Theme.toneGood.g, Theme.toneGood.b, 0.4)

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight + 2

            Rectangle {
                width: 5
                height: 28
                radius: 2.5
                color: Theme.toneGood
                anchors.verticalCenter: parent.verticalCenter
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 100
                spacing: 2

                Text {
                    width: parent.width
                    text: panel.currentEvent ? panel.currentEvent.summary : ""
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: panel.currentEvent
                        ? (panel.currentEvent.location
                            ? panel.currentEvent.location
                            : "in progress")
                        : ""
                    color: Theme.textDim
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                    elide: Text.ElideRight
                }
            }
        }
    }

    Text {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing
        visible: panel.events.length === 0 && panel.currentEvent === null
        text: "Nothing scheduled. Point the calendar at a directory of .ics files."
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontSmall
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
    }

    ListView {
        id: list
        anchors.top: currentCard.visible ? currentCard.bottom : header.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 2
        model: panel.hours

        delegate: Item {
            id: slot
            required property var modelData

            width: list.width
            height: Math.max(30, column.implicitHeight + 10)

            // The hour label and the rule.
            Row {
                id: labelRow
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.topMargin: 4
                spacing: Theme.spacingTight + 2

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Format.hourLabel(slot.modelData.start)
                    color: slot.modelData.isNow ? Theme.accent : Theme.textFaint
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontTiny
                    font.weight: slot.modelData.isNow ? Font.DemiBold : Font.Normal
                }

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 34
                    height: 1
                    color: Theme.border
                    visible: slot.modelData.isNow
                }
            }

            Column {
                id: column
                anchors.left: parent.left
                anchors.leftMargin: 52
                anchors.right: parent.right
                anchors.top: labelRow.bottom
                anchors.topMargin: 3
                spacing: 4

                Repeater {
                    model: slot.modelData.events

                    Card {
                        required property var modelData

                        width: column.width
                        height: 42
                        radius: Theme.radiusSmall

                        Row {
                            anchors.fill: parent
                            anchors.margins: Theme.spacingTight + 1
                            spacing: Theme.spacingTight + 2

                            Rectangle {
                                width: 3
                                height: parent.height - 12
                                radius: 1.5
                                color: Theme.toneInfo
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Column {
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - 90
                                spacing: 1

                                Text {
                                    width: parent.width
                                    text: modelData.summary
                                    color: Theme.text
                                    font.family: Theme.fontFamily
                                    font.pixelSize: Theme.fontSmall
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }

                                Text {
                                    width: parent.width
                                    visible: modelData.location !== "" || modelData.description !== ""
                                    text: modelData.location
                                        ? modelData.location
                                        : modelData.description
                                    color: Theme.textDim
                                    font.family: Theme.fontFamily
                                    font.pixelSize: Theme.fontTiny
                                    elide: Text.ElideRight
                                }
                            }

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: Format.timeOfDay(modelData.start_unix)
                                color: Theme.textFaint
                                font.family: Theme.monoFamily
                                font.pixelSize: Theme.fontTiny
                            }
                        }
                    }
                }
            }
        }
    }
}
