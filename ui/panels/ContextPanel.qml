import QtQuick
import ".."
import "../components"

// The home panel.
//
// "Only important events and context happening in the machine" — so this is
// not a system monitor. It is a short list of what is going on, with the
// vitals underneath it as context rather than as the subject.
Item {
    id: panel

    property var bridge: null

    readonly property var stats: bridge && bridge.stats ? bridge.stats : null
    readonly property var session: bridge && bridge.snapshot ? bridge.snapshot.session : null
    readonly property var ambient: bridge && bridge.activity ? bridge.activity : null
    readonly property var recent: bridge && bridge.activities ? bridge.activities : []
    readonly property var todo: bridge && bridge.todos && bridge.todos.length > 0 ? bridge.todos[0] : null
    readonly property var timer: bridge && bridge.timer ? bridge.timer : null

    /// The things worth showing: the persistent condition, and anything that
    /// happened in the last few minutes that has not faded out.
    readonly property var highlights: {
        var list = [];
        if (ambient)
            list.push(ambient);
        for (var i = 0; i < recent.length; i++) {
            if (list.length >= 4)
                break;
            var item = recent[i];
            if (!item)
                continue;
            if (item.sticky)
                continue;
            list.push(item);
        }
        return list;
    }

    Column {
        id: column
        anchors.fill: parent
        anchors.margins: Theme.spacing
        anchors.bottomMargin: Theme.spacingTight
        spacing: Theme.spacing

        // ---- vitals ----

        Grid {
            width: parent.width
            columns: 4
            columnSpacing: Theme.spacing
            rowSpacing: Theme.spacing

            StatTile {
                width: (parent.width - Theme.spacing * 3) / 4
                label: "CPU"
                value: panel.stats ? panel.stats.cpu_percent.toFixed(0) + "%" : "—"
                detail: panel.stats && panel.stats.load_average !== null
                    ? "load " + panel.stats.load_average.toFixed(2)
                    : ""
                tone: panel.stats && panel.stats.cpu_percent > 80 ? Theme.toneWarn : Theme.text
            }

            StatTile {
                width: (parent.width - Theme.spacing * 3) / 4
                label: "MEMORY"
                value: panel.stats
                    ? Format.bytesToGiB(panel.stats.memory_used_bytes) + "G"
                    : "—"
                detail: panel.stats
                    ? "of " + Format.bytesToGiB(panel.stats.memory_total_bytes) + "G"
                    : ""
            }

            StatTile {
                width: (parent.width - Theme.spacing * 3) / 4
                label: "UPTIME"
                value: panel.stats ? Format.uptime(panel.stats.uptime_secs) : "—"
                detail: panel.session ? panel.session.compositor : ""
                tone: Theme.textDim
            }

            StatTile {
                width: (parent.width - Theme.spacing * 3) / 4
                label: "UNREAD"
                value: bridge ? String(bridge.unreadCount) : "0"
                detail: bridge && bridge.notifications ? bridge.notifications.length + " total" : ""
                tone: bridge && bridge.unreadCount > 0 ? Theme.toneWarn : Theme.textDim
            }
        }

        // ---- what is happening ----

        SectionHeader {
            width: parent.width
            title: "Happening now"
            note: panel.highlights.length > 0 ? panel.highlights.length + " ACTIVE" : "ALL QUIET"
        }

        Item {
            width: parent.width
            height: Math.max(40, panel.highlights.length * 44)

            Text {
                anchors.centerIn: parent
                visible: panel.highlights.length === 0
                text: "Nothing has asked for your attention."
                color: Theme.textFaint
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontBody
            }

            Column {
                anchors.fill: parent
                spacing: 6

                Repeater {
                    model: panel.highlights

                    Card {
                        required property var modelData
                        required property int index

                        width: parent.width
                        height: 40
                        radius: Theme.radiusSmall

                        Row {
                            id: activityRow
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.spacingTight + 2
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.spacingTight + 2
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: Theme.spacingTight + 2

                            Rectangle {
                                id: activityDot
                                width: 6
                                height: 6
                                radius: 3
                                color: Theme.toneColor(modelData.tone ? modelData.tone : "neutral")
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Icon {
                                id: activityIcon
                                anchors.verticalCenter: parent.verticalCenter
                                name: modelData.glyph ? modelData.glyph : "spark"
                                width: 14
                                height: 14
                                color: Theme.toneColor(modelData.tone ? modelData.tone : "neutral")
                            }

                            Text {
                                id: activityTitle
                                anchors.verticalCenter: parent.verticalCenter
                                // The title takes what the subtitle has not
                                // claimed, so the row cannot overflow the card
                                // and clip the subtitle at the edge.
                                width: Math.max(0, activityRow.width
                                    - activityDot.width - activityIcon.width
                                    - activitySubtitle.width - activityRow.spacing * 3)
                                text: modelData.title ? modelData.title : ""
                                color: Theme.text
                                font.family: Theme.fontFamily
                                font.pixelSize: Theme.fontBody
                                font.weight: Font.Medium
                                elide: Text.ElideRight
                            }

                            Text {
                                id: activitySubtitle
                                anchors.verticalCenter: parent.verticalCenter
                                text: modelData.subtitle ? modelData.subtitle : ""
                                color: Theme.textFaint
                                font.family: Theme.fontFamily
                                font.pixelSize: Theme.fontTiny
                                elide: Text.ElideRight
                                width: Math.min(implicitWidth, activityRow.width * 0.4)
                            }
                        }
                    }
                }
            }
        }

        // ---- the timer and the next thing ----

        Row {
            width: parent.width
            spacing: Theme.spacing

            Card {
                width: (parent.width - Theme.spacing) / 2
                height: 74

                Row {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingTight + 4
                    spacing: Theme.spacingTight + 2

                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        name: "timer"
                        width: 17
                        height: 17
                        color: panel.timer && panel.timer.running ? Theme.toneGood : Theme.textFaint
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2
                        width: parent.width - 30

                        Text {
                            width: parent.width
                            text: panel.timer && panel.timer.phase && panel.timer.phase !== "idle"
                                ? panel.timer.phase
                                : "No timer running"
                            color: Theme.text
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: panel.timer && panel.timer.remaining_ms
                                ? Format.duration(panel.timer.remaining_ms) + " left"
                                : "start one from the todos panel"
                            color: Theme.textDim
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.fontTiny
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Card {
                width: (parent.width - Theme.spacing) / 2
                height: 74

                Row {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingTight + 4
                    spacing: Theme.spacingTight + 2

                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        name: "check-square"
                        width: 17
                        height: 17
                        color: panel.todo ? Theme.toneWarn : Theme.textFaint
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2
                        width: parent.width - 30

                        Text {
                            width: parent.width
                            text: panel.todo ? panel.todo.text : "Nothing scheduled"
                            color: Theme.text
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: panel.todo && panel.todo.due_unix
                                ? "due " + Format.until(panel.todo.due_unix)
                                : "no deadline"
                            color: Theme.textDim
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.fontTiny
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }
}
