import QtQuick
import "panels"

// The panel router.
//
// One view is visible at a time. Every panel is instantiated from the start
// so that it can keep its own state — a half-written todo, a note being
// edited — but only the visible one draws anything heavy.
Item {
    id: host

    property int view: 0
    property var bridge: null

    readonly property var isled: bridge

    ContextPanel {
        anchors.fill: parent
        visible: host.view === 0
        enabled: host.view === 0
        bridge: host.bridge
    }

    MediaPanel {
        anchors.fill: parent
        visible: host.view === 1
        enabled: host.view === 1
        bridge: host.bridge
    }

    NotificationsPanel {
        anchors.fill: parent
        visible: host.view === 2
        enabled: host.view === 2
        bridge: host.bridge
    }

    ClipboardPanel {
        anchors.fill: parent
        visible: host.view === 3
        enabled: host.view === 3
        bridge: host.bridge
    }

    CalendarPanel {
        anchors.fill: parent
        visible: host.view === 4
        enabled: host.view === 4
        bridge: host.bridge
    }

    TodosPanel {
        anchors.fill: parent
        visible: host.view === 5
        enabled: host.view === 5
        bridge: host.bridge
    }

    NotesPanel {
        anchors.fill: parent
        visible: host.view === 6
        enabled: host.view === 6
        bridge: host.bridge
    }

    NetworkPanel {
        anchors.fill: parent
        visible: host.view === 7
        enabled: host.view === 7
        bridge: host.bridge
    }

    ThemePanel {
        anchors.fill: parent
        visible: host.view === 8
        enabled: host.view === 8
        bridge: host.bridge
    }

    TrayPanel {
        anchors.fill: parent
        visible: host.view === 9
        enabled: host.view === 9
        bridge: host.bridge
    }
}
