import QtQuick

Rectangle {
    id: island

    property bool expanded: false
    property int view: 0
    property bool animateSelf: true
    property bool showFocusProbe: true
    property bool showStatus: false

    readonly property bool probeHasFocus: probe.hasFocus
    property string status: ""

    readonly property int targetWidth: expanded ? Theme.panelWidth : Theme.compactWidth
    readonly property int targetHeight: expanded ? Theme.panelHeight : Theme.compactHeight

    width: animateSelf ? targetWidth : implicitWidth
    height: animateSelf ? targetHeight : implicitHeight
    implicitWidth: targetWidth
    implicitHeight: targetHeight

    color: Theme.surface
    radius: expanded ? Theme.radiusPanel : Theme.radiusPill
    border.width: 1
    border.color: Theme.border

    Behavior on width {
        NumberAnimation {
            duration: Theme.morphDuration
            easing.type: Easing.OutQuart
        }
    }
    Behavior on height {
        NumberAnimation {
            duration: Theme.morphDuration
            easing.type: Easing.OutQuart
        }
    }
    Behavior on radius {
        NumberAnimation {
            duration: Theme.morphDuration
            easing.type: Easing.OutQuart
        }
    }

    MouseArea {
        anchors.fill: parent
        z: -1
        onClicked: island.expanded = !island.expanded
    }

    Text {
        id: label

        anchors.centerIn: parent
        visible: !island.expanded
        color: Theme.text
        font.family: "Inter"
        font.pixelSize: 14
        font.weight: Font.DemiBold
        text: island.status.length > 0 ? island.status : "compact pill"
    }

    PanelHost {
        id: panel

        anchors.fill: parent
        view: island.view
        opacity: island.expanded ? 1 : 0
        visible: opacity > 0
        enabled: opacity > 0

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.crossfadeDuration
                easing.type: Easing.OutCubic
            }
        }
    }

    FocusProbeItem {
        id: probe

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacingLoose
        width: Math.min(440, parent.width - Theme.spacingLoose * 2)
        height: 44
        opacity: island.expanded ? 1 : 0
        visible: opacity > 0
        enabled: opacity > 0

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.crossfadeDuration
                easing.type: Easing.OutCubic
            }
        }
    }

    function collapse() {
        expanded = false;
        probe.releaseFocus();
    }

    function grabFocus() {
        setExpanded(true);
        probe.grabFocus();
    }

    function setExpanded(value) {
        expanded = value;
        if (!value)
            probe.releaseFocus();
    }

    Keys.onEscapePressed: {
        island.collapse();
        event.accepted = true;
    }
    Keys.onLeftPressed: {
        island.cycleView(-1);
        event.accepted = true;
    }
    Keys.onRightPressed: {
        island.cycleView(1);
        event.accepted = true;
    }

    function cycleView(delta) {
        var count = PanelCatalog.names.length;
        view = (view + delta + count) % count;
    }
}