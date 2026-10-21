import QtQuick
import Quickshell

// The root surface.
//
// One layer-shell surface, full width, transparent. The island is an item
// inside it that changes shape; the surface itself does not resize, so the
// compositor is never asked to reconfigure it mid-animation.
//
// Quickshell's `PanelWindow` has no `namespace` and no input region, so the
// surface is anchored `top`/`left`/`right` and the island is centred inside
// it by hand.
PanelWindow {
    id: root

    color: "transparent"

    exclusionMode: ExclusionMode.Ignore
    focusable: IslandSettings.keyboardFocus

    property int view: 0
    readonly property int viewCount: PanelCatalog.names.length

    // The surface is always the full width and tall enough for the tallest
    // panel, so opening a panel never resizes it.
    implicitWidth: screen.width
    implicitHeight: Theme.panelHeight + 80

    anchors {
        top: true
        left: true
        right: true
    }

    margins {
        top: 10
        left: 0
    }

    Island {
        id: island
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        view: root.view
        bridge: root.isled
        ambient: root.isled.activity

        onViewRequested: (index) => root.showView(index)
        onViewStepped: (delta) => root.stepView(delta)
    }

    // The bridge. A property rather than a child object, because a QtObject
    // declared as a child of a PanelWindow is never instantiated.
    property IsledBridge isled: IsledBridge {
        onToggleRequested: root.toggleIsland()
        onViewRequested: (index) => root.showView(index)
        onViewStepped: (delta) => root.stepView(delta)
        onFocusRequested: root.focusIsland()
        onActivityTriggered: (activity) => island.triggerWith(activity)
    }

    // ---- routing ----

    function showView(index) {
        view = PanelCatalog.clamp(index);
        island.open();
    }

    function stepView(delta) {
        showView((view + delta + viewCount) % viewCount);
    }

    function toggleIsland() {
        if (island.mode === "full")
            island.collapse();
        else
            island.open();
    }

    function focusIsland() {
        island.open();
    }

    Component.onCompleted: {
        root.isled.call("ping");
    }
}
