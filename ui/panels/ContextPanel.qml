import QtQuick
import ".."

Item {
    id: panel

    property var stats: ({})
    property var snapshot: ({})

    readonly property var session: snapshot.session ?? {}

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing

        Text {
            text: "At a glance"
            color: Theme.text
            font.family: "Inter"
            font.pixelSize: 24
            font.weight: Font.Bold
        }

        Text {
            text: (session.compositor ?? "unknown") + " · wayland " + (session.wayland_display ?? "?")
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 13
        }

        Grid {
            columns: 3
            columnSpacing: Theme.spacing
            rowSpacing: Theme.spacing
            width: parent.width

            Repeater {
                model: [
                    {
                        title: "Cpu",
                        value: stats.cpu_percent !== undefined ? stats.cpu_percent.toFixed(1) + "%" : "—"
                    },
                    {
                        title: "Memory",
                        value: stats.memory_used_bytes !== undefined
                            ? Format.bytesToGiB(stats.memory_used_bytes) + " / " + Format.bytesToGiB(stats.memory_total_bytes) + " GiB"
                            : "—"
                    },
                    {
                        title: "Uptime",
                        value: Format.uptime(stats.uptime_secs)
                    }
                ]

                delegate: Rectangle {
                    required property var modelData

                    width: (panel.width - Theme.spacing * 2) / 3
                    height: 82
                    color: Theme.surfaceRaised
                    radius: 14
                    border.width: 1
                    border.color: Theme.border

                    Column {
                        anchors.centerIn: parent
                        spacing: Theme.spacingTight

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 11
                            text: modelData.title
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            color: Theme.text
                            font.family: "Inter"
                            font.pixelSize: 17
                            font.weight: Font.DemiBold
                            text: modelData.value
                        }
                    }
                }
            }
        }

        Flow {
            width: parent.width
            spacing: Theme.spacingTight

            Repeater {
                model: [
                    {
                        label: stats.load_average ? "Load " + stats.load_average.toFixed(2) : "Load —",
                        dim: !stats.load_average
                    },
                    {
                        label: (session.compositor ?? "unknown").toUpperCase(),
                        dim: false
                    },
                    {
                        label: "pid " + (snapshot.pid ?? "?"),
                        dim: true
                    }
                ]

                delegate: Rectangle {
                    required property var modelData

                    height: 22
                    radius: 11
                    color: Theme.surfaceRaised
                    border.width: 1
                    border.color: Theme.border
                    width: chipText.implicitWidth + 20

                    Text {
                        id: chipText
                        anchors.centerIn: parent
                        color: modelData.dim ? Theme.textDim : Theme.text
                        font.family: "Inter"
                        font.pixelSize: 11
                        text: modelData.label
                    }
                }
            }
        }

        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 11
            text: "Every stat above arrived over the isled socket as an unsolicited event. Nothing in the UI reads the system directly."
        }

        Item {
            width: parent.width
            height: 1
        }
    }
}
