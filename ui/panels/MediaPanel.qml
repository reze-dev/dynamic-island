import QtQuick
import Quickshell.Services.Mpris
import ".."
import "../components"

// Now playing.
//
// This one reads MPRIS directly, through Quickshell's own service, rather
// than asking the daemon. The daemon is the right place for state the shell
// owns; a media player is somebody else's state and the bus already has it.
Item {
    id: panel

    property var bridge: null

    // The model fills in asynchronously after the bus is read, so this has to
    // be a binding and not a one-shot read: checking it in onCompleted sees
    // an empty list and never updates.
    readonly property var players: Mpris.players.values
    readonly property bool hasPlayer: panel.players != null && panel.players.length > 0

    // QML evaluates bindings whether or not an item is visible, so every read
    // goes through a stand-in rather than a null check at each use.
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

    function mmss(seconds) {
        if (seconds === undefined || seconds === null || !isFinite(seconds) || seconds < 0)
            return "0:00";
        var total = Math.floor(seconds);
        var minutes = Math.floor(total / 60);
        var secs = total % 60;
        return minutes + ":" + (secs < 10 ? "0" + secs : "" + secs);
    }

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "Now playing"
        note: panel.hasPlayer ? "MPRIS" : ""
    }

    // The empty state — and why it is empty.
    Column {
        anchors.centerIn: parent
        spacing: Theme.spacingTight
        visible: !panel.hasPlayer

        Icon {
            anchors.horizontalCenter: parent.horizontalCenter
            name: "music"
            width: 24
            height: 24
            color: Theme.onSurfaceVariant
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Nothing is playing."
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodySmall.size
            font.weight: Theme.type.bodySmall.weight
            font.letterSpacing: Theme.type.bodySmall.tracking
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Open a player that speaks MPRIS (VLC, mpv, Spotify, …)"
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
        }
    }

    // The player.
    Item {
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        visible: panel.hasPlayer

        // The track: the cover, and what it is.
        Item {
            id: card
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 58

            Rectangle {
                id: art
                width: 58
                height: 58
                radius: Theme.radiusMedium
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                color: Theme.surfaceFillLow
                border.width: 1
                border.color: Theme.outlineVariant
                clip: true

                Image {
                    anchors.fill: parent
                    source: panel.p.trackArtUrl ? panel.p.trackArtUrl : ""
                    asynchronous: true
                    fillMode: Image.PreserveAspectCrop

                    onStatusChanged: {
                        if (status === Image.Error)
                            source = "";
                    }
                }

                Icon {
                    anchors.centerIn: parent
                    visible: panel.p.trackArtUrl === ""
                    name: "disc"
                    width: 28
                    height: 28
                    color: Theme.onSurfaceVariant
                }
            }

            Column {
                anchors.left: art.right
                anchors.leftMargin: Theme.spacingTight + 4
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Text {
                    width: parent.width
                    text: panel.p.trackTitle ? panel.p.trackTitle : "Unknown track"
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.titleMedium.size
                    font.weight: Theme.type.titleMedium.weight
                    font.letterSpacing: Theme.type.titleMedium.tracking
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: panel.p.trackArtist
                        ? (panel.p.identity !== "" ? panel.p.trackArtist + " · " + panel.p.identity : panel.p.trackArtist)
                        : panel.p.identity
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.bodySmall.size
                    font.weight: Theme.type.bodySmall.weight
                    font.letterSpacing: Theme.type.bodySmall.tracking
                    elide: Text.ElideRight
                }
            }
        }

        // The progress bar.
        Item {
            id: progress
            anchors.top: card.bottom
            anchors.topMargin: Theme.spacingLoose
            anchors.left: parent.left
            anchors.right: parent.right
            height: 20

            Rectangle {
                id: track
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                height: 4
                radius: height / 2
                color: Theme.surfaceFillHighest
            }

            Rectangle {
                id: fill
                anchors.left: track.left
                anchors.verticalCenter: track.verticalCenter
                height: track.height
                radius: track.radius
                width: panel.p.lengthSupported && panel.p.length > 0
                    ? track.width * Math.min(1, panel.p.position / panel.p.length)
                    : 0
                color: Theme.primary

                Behavior on width {
                    NumberAnimation {
                        duration: 700
                        easing: Theme.decelerate
                    }
                }
            }

            Row {
                anchors.top: track.bottom
                anchors.topMargin: 6
                anchors.left: parent.left
                anchors.right: parent.right

                Text {
                    text: panel.mmss(panel.p.position)
                    color: Theme.onSurfaceVariant
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                }

                Item {
                    width: Math.max(0, parent.width - 80)
                }

                Text {
                    text: panel.p.lengthSupported ? panel.mmss(panel.p.length) : "--:--"
                    color: Theme.onSurfaceVariant
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                }
            }
        }

        // The transport.
        Row {
            anchors.bottom: parent.bottom
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.spacingLoose

            IconButton {
                icon: "prev"
                size: 40
                enabled: panel.p.canGoPrevious
                tooltip: "Previous"
                onClicked: panel.p.canGoPrevious ? panel.p.previous() : null
            }

            IconButton {
                icon: panel.p.isPlaying ? "pause" : "play"
                size: 48
                variant: "filled"
                enabled: panel.p.canTogglePlaying
                tooltip: panel.p.isPlaying ? "Pause" : "Play"
                onClicked: panel.p.canTogglePlaying ? panel.p.togglePlaying() : null
            }

            IconButton {
                icon: "next"
                size: 40
                enabled: panel.p.canGoNext
                tooltip: "Next"
                onClicked: panel.p.canGoNext ? panel.p.next() : null
            }
        }
    }
}
