import QtQuick
import Quickshell

PanelWindow {
    id: root

    color: "transparent"

    exclusionMode: ExclusionMode.Ignore
    focusable: IslandSettings.keyboardFocus

    property bool expanded: false
    property int view: 0

    readonly property bool surfaceFollowsIsland: IslandSettings.morphApproach === 0

    implicitWidth: surfaceFollowsIsland ? island.width : screen.width
    implicitHeight: island.height

    anchors {
        top: true
        left: true
        right: !root.surfaceFollowsIsland
    }
    margins {
        top: 8
    }

    Island {
        id: island
        anchors.horizontalCenter: parent.horizontalCenter
        expanded: root.expanded
        view: root.view
        animateSelf: !root.surfaceFollowsIsland
    }

    property IsledBridge isled: IsledBridge {
        id: bridge

        onToggleRequested: root.expanded = !root.expanded
        onViewRequested: (index) => {
            root.view = index;
            root.expanded = true;
        }
    }

    Loader {
        anchors.fill: parent
        sourceComponent: root.isled.connected ? statusOverlay : null
    }

    Component {
        id: statusOverlay

        Item {
            anchors.fill: parent

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Escape) {
                    root.expanded = false;
                    event.accepted = true;
                }
            }

            Text {
                anchors.centerIn: parent
                color: Theme.textDim
                font.family: "Inter"
                font.pixelSize: 11
                text: root.isled.connected
                    ? "isled · " + (root.isled.snapshot?.session?.compositor ?? "?") + " · ticks " + root.isled.tickCount
                    : "isled not reachable"
            }
        }
    }

    Component.onCompleted: {
        root.isled.call("ping")
    }
}