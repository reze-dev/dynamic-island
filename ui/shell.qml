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
    focusable: IslandSettings.keyboardFocus

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

    // The top bar: what the machine is called, and what time it is.
    //
    // The mock draws these straight onto the page, which works because the
    // page is black. The shell floats over whatever is on the screen, so
    // they get a plate of their own: a small piece of glass that reads
    // against a bright wallpaper as well as a dark one.
    Item {
        id: bar
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: 14
        height: 28

        Rectangle {
            id: brandPlate
            anchors.left: parent.left
            anchors.leftMargin: 25
            anchors.verticalCenter: parent.verticalCenter
            width: brandRow.width + 24
            height: 28
            radius: height / 2
            color: Theme.glass
            border.width: 1
            border.color: Theme.hairline

            Row {
                id: brandRow
                anchors.centerIn: parent
                spacing: 8

                Rectangle {
                    width: 8
                    height: 8
                    radius: 4
                    color: Theme.primary
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "ctOS"
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelMedium.size
                    font.weight: Theme.type.labelMedium.weight
                    font.letterSpacing: Theme.overlineTracking
                }
            }
        }

        Rectangle {
            id: clockPlate
            anchors.right: parent.right
            anchors.rightMargin: 25
            anchors.verticalCenter: parent.verticalCenter
            width: clockText.implicitWidth + 24
            height: 28
            radius: height / 2
            color: Theme.glass
            border.width: 1
            border.color: Theme.hairline

            Text {
                id: clockText
                anchors.centerIn: parent
                text: Format.clock()
                color: Theme.onSurfaceVariant
                font.family: Theme.monoFamily
                font.pixelSize: Theme.type.labelMedium.size
                font.weight: Theme.type.labelMedium.weight
                font.letterSpacing: Theme.type.labelMedium.tracking
            }
        }

        Timer {
            interval: 1000
            running: true
            repeat: true
            onTriggered: clockText.text = Format.clock()
        }
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
