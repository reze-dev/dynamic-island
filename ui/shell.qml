import QtQuick
import Quickshell
import Quickshell.Wayland
import "components"

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

    // The keyboard belongs to the island only while a panel is open.
    //
    // Niri hands the keyboard to a newly mapped on-demand layer surface, so
    // a surface that is focusable from the start takes the keyboard away
    // from the terminal at login and holds it until something else is
    // clicked. A notification that opens a panel must not do that either:
    // the keyboard has to stay where the reader left it.
    focusable: IslandSettings.keyboardFocus && island.mode === "full"

    // The clickthrough mask. The surface is as wide as the screen and as
    // tall as the tallest panel, so without a mask every click in that
    // corner of the screen belongs to the island instead of to the window
    // underneath it. It also keeps the keyboard: niri gives the keyboard
    // back to whatever was clicked, so a click that lands on the window
    // below is what returns it.
    mask: Region {
        regions: [
            Region {
                item: island
                radius: island.radius
            },
            Region {
                item: snackbar
            }
        ]
    }

    property int view: 0
    readonly property int viewCount: PanelCatalog.names.length

    // The surface is always the full width and tall enough for the tallest
    // panel, so opening a panel never resizes it.
    implicitWidth: screen.width
    implicitHeight: Theme.panelMaxHeight + 80

    anchors {
        top: true
        left: true
        right: true
    }

    margins {
        top: 10
        left: 0
    }

    // The compositor blurs whatever is behind the island, which is what
    // makes the open panel glass rather than a flat sheet. Only the open
    // panel gets it: the idle island is a hole in the screen and there is
    // nothing behind it worth blurring.
    BackgroundEffect.blurRegion: island.displayState === "full" ? blur : null

    property Region blur: Region {
        item: island
        radius: island.radius
    }

    Island {
        id: island
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        view: root.view
        bridge: root.isled
        ambient: root.isled.activity
        maxHeight: Math.min(Theme.panelMaxHeight, screen.height - Theme.panelScreenMargin)

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
        onNoticeRaised: (message, action) => root.notify(message, action)
    }

    // The snackbar. It sits under the island, and it is the shell's way of
    // saying that something happened. It follows the island's bottom edge,
    // the way the mock's toast does, so it turns up under a short island
    // and under a tall one alike.
    Snackbar {
        id: snackbar
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: island.bottom
        anchors.topMargin: 20
    }

    function notify(message, action) {
        snackbar.open(message, action);
    }

    // The island asks for the keyboard when the panel opens. The click
    // that opened it arrived before the window was focusable, so the
    // request in the island's own click handler went nowhere; this one
    // comes after, and it is what makes Esc reach the island at all.
    onFocusableChanged: {
        if (root.focusable)
            island.forceActiveFocus();
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
