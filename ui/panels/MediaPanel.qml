import QtQuick
import Quickshell.Services.Mpris
import ".."

Rectangle {
    id: panel

    // The model fills in asynchronously after the bus is read, so this has to be
    // a binding and not a one-shot read: checking it in onCompleted sees an
    // empty list and never updates.
    readonly property var players: Mpris.players.values
    readonly property bool hasPlayer: panel.players != null && panel.players.length > 0

    // QML evaluates bindings whether or not an item is visible, so every read
    // goes through a stand-in rather than a null check at each use. A plain
    // object beats a QtObject property here: it has no construction order to
    // get wrong and cannot be null.
    readonly property var p: panel.hasPlayer ? panel.players[0] : {
        "identity": "",
        "trackTitle": "",
        "trackArtist": "",
        "trackAlbum": "",
        "position": 0,
        "length": 0,
        "isPlaying": false,
        "lengthSupported": false,
        "trackArtUrl": "",
        "canGoPrevious": false,
        "canGoNext": false,
        "canTogglePlaying": false
    }

    color: "transparent"

    function mmss(seconds) {
        if (seconds === undefined || seconds === null || !isFinite(seconds) || seconds < 0)
            return "0:00";
        var total = Math.floor(seconds);
        var minutes = Math.floor(total / 60);
        var secs = total % 60;
        return minutes + ":" + (secs < 10 ? "0" + secs : "" + secs);
    }

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing

        Row {
            width: parent.width
            height: 30

            Text {
                text: "Now playing"
                color: Theme.text
                font.family: "Inter"
                font.pixelSize: 24
                font.weight: Font.Bold
            }

            Item {
                width: Math.max(0, parent.width - 200)
                height: 30
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                visible: panel.hasPlayer
                height: 20
                radius: 10
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.border
                width: sourceText.implicitWidth + 16

                Text {
                    id: sourceText
                    anchors.centerIn: parent
                    color: Theme.textDim
                    font.family: "Inter"
                    font.pixelSize: 11
                    text: panel.p.identity
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 76
            radius: 20
            color: Theme.surfaceRaised
            border.width: 1
            border.color: Theme.border
            visible: !panel.hasPlayer

            Text {
                anchors.centerIn: parent
                color: Theme.textDim
                font.family: "Inter"
                font.pixelSize: 13
                text: "No MPRIS player. Start something and it appears here."
            }
        }

        Rectangle {
            width: parent.width
            height: 132
            radius: 20
            color: Theme.surfaceRaised
            border.width: 1
            border.color: Theme.border
            visible: panel.hasPlayer

            Row {
                anchors.fill: parent
                anchors.margins: Theme.spacing
                spacing: Theme.spacing

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 92
                    height: 92
                    radius: 30
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.border

                    Image {
                        id: art
                        anchors.fill: parent
                        anchors.margins: 5
                        fillMode: Image.PreserveAspectFit
                        asynchronous: true
                        cache: true
                        sourceSize.width: 192
                        sourceSize.height: 192
                        visible: status === Image.Ready
                        source: panel.p.trackArtUrl ? panel.p.trackArtUrl : ""
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: 30
                        visible: art.status !== Image.Ready
                        color: Qt.rgba(1, 1, 1, 0.03)

                        Text {
                            anchors.centerIn: parent
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 26
                            text: "♪"
                        }
                    }
                }

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - 92 - Theme.spacing)
                    spacing: 5

                    Text {
                        width: parent.width
                        color: Theme.text
                        font.family: "Inter"
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        text: panel.p.trackTitle !== "" ? panel.p.trackTitle : "Unknown title"
                    }

                    Text {
                        width: parent.width
                        color: Theme.textDim
                        font.family: "Inter"
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        text: panel.p.trackArtist !== "" ? panel.p.trackArtist : panel.p.trackAlbum
                    }

                    Rectangle {
                        width: parent.width
                        height: 4
                        radius: 2
                        color: Qt.rgba(1, 1, 1, 0.09)
                        visible: panel.p.length > 0

                        Rectangle {
                            width: parent.width * (panel.p.length > 0
                                ? Math.min(1, panel.p.position / panel.p.length)
                                : 0)
                            height: parent.height
                            radius: 2
                            color: Theme.accent

                            Behavior on width {
                                NumberAnimation {
                                    duration: 700
                                    easing.type: Easing.Linear
                                }
                            }
                        }
                    }

                    Row {
                        width: parent.width
                        visible: panel.p.length > 0

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 10
                            text: panel.mmss(panel.p.position)
                        }

                        Item {
                            width: Math.max(0, parent.width - 90)
                            height: 10
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 10
                            text: panel.mmss(panel.p.length)
                        }
                    }
                }
            }
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.spacing
            visible: panel.hasPlayer

            Rectangle {
                width: 38
                height: 38
                radius: 19
                opacity: panel.p.canGoPrevious ? 1 : 0.35
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.border

                Text {
                    anchors.centerIn: parent
                    color: Theme.text
                    font.family: "Inter"
                    font.pixelSize: 14
                    text: "‹‹"

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.players[0].previous()
                    }
                }
            }

            Rectangle {
                width: 52
                height: 52
                radius: 26
                opacity: panel.p.canTogglePlaying ? 1 : 0.35
                color: Theme.accent

                Text {
                    anchors.centerIn: parent
                    color: "#0b0e15"
                    font.family: "Inter"
                    font.pixelSize: 17
                    text: panel.p.isPlaying ? "❙❙" : "▶"

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.players[0].togglePlaying()
                    }
                }
            }

            Rectangle {
                width: 38
                height: 38
                radius: 19
                opacity: panel.p.canGoNext ? 1 : 0.35
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.border

                Text {
                    anchors.centerIn: parent
                    color: Theme.text
                    font.family: "Inter"
                    font.pixelSize: 14
                    text: "››"

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.players[0].next()
                    }
                }
            }
        }
    }
}
