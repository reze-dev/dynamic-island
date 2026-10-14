import QtQuick
import Quickshell

PanelWindow {
    id: root

    color: "transparent"

    exclusionMode: ExclusionMode.Ignore
    focusable: IslandSettings.keyboardFocus

    property int view: 0
    readonly property int viewCount: PanelCatalog.names.length

    readonly property bool surfaceFollowsIsland: IslandSettings.morphApproach === 0

    implicitWidth: surfaceFollowsIsland ? island.width : screen.width
    implicitHeight: island.height

    anchors {
        top: true
        left: !root.surfaceFollowsIsland
        right: !root.surfaceFollowsIsland
    }
    margins {
        top: 8
        left: root.surfaceFollowsIsland
            ? Math.round((screen.width - island.width) / 2)
            : 0
    }

    Island {
        id: island
        anchors.horizontalCenter: parent.horizontalCenter
        view: root.view
        animateSelf: !root.surfaceFollowsIsland
        status: root.isled.connected
            ? "isled · " + (root.isled.snapshot?.session?.compositor ?? "?") + " · " + root.isled.tickCount
            : "isled offline"
    }

    function setExpanded(value) {
        island.setExpanded(value);
    }

    function toggleIsland() {
        island.setExpanded(!island.expanded);
    }

    function focusProbe() {
        island.grabFocus();
    }

    function showView(index) {
        view = PanelCatalog.clamp(index);
        setExpanded(true);
    }

    function stepView(delta) {
        showView((view + delta + viewCount) % viewCount);
    }

    property IsledBridge isled: IsledBridge {
        onToggleRequested: root.toggleIsland()
        onViewRequested: (index) => {
            root.showView(index);
        }
        onViewStepped: (delta) => root.stepView(delta)
        onFocusRequested: root.focusProbe()
    }

    Component.onCompleted: {
        root.isled.call("ping")
    }
}
