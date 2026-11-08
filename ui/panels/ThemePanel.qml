import QtQuick
import ".."
import "../components"

// The wallpaper switcher, and the theme that goes with it.
//
// The catalog is a directory of images the daemon owns; the engine is a
// separate process that draws them. A wallpaper can be applied by clicking
// it, the rotation can be left running, and the directory can be pointed
// somewhere else.
Item {
    id: panel

    property var bridge: null
    property string dirDraft: ""
    property bool dirEditing: false

    // The Wallhaven tab: a remote grid the daemon fetches for us, because
    // Quickshell's QML cannot load remote images itself.
    property string tab: "local"
    property var whItems: []
    property int whPage: 1
    property int whLastPage: 1
    property bool whLoading: false
    property string whError: ""
    property string whQuery: ""
    // The handoff tab: the daemon watches ~/Downloads and imports saves.
    property string handoffDir: ""
    property int handoffPending: 0
    property string handoffUrl: ""
    property Timer whDebounce: Timer {
        id: whTimer
        interval: 400
        repeat: false
        onTriggered: panel.whSearch()
    }

    function whSearch() {
        if (!panel.bridge || panel.tab !== "wallhaven")
            return;
        panel.whLoading = true;
        panel.whError = "";
        panel.bridge.call(
            "wallpaper.search",
            { "q": panel.whQuery, "page": panel.whPage },
            function (ok, payload, error) {
                panel.whLoading = false;
                if (ok && payload) {
                    panel.whItems = payload.items || [];
                    panel.whPage = payload.page || 1;
                    panel.whLastPage = payload.last_page || 1;
                } else {
                    panel.whError = error || "Wallhaven is unreachable.";
                }
            });
    }

    function handoffStatus() {
        if (!panel.bridge || panel.tab !== "imported")
            return;
        panel.bridge.call(
            "wallpaper.handoff-status",
            {},
            function (ok, payload, error) {
                if (ok && payload) {
                    panel.handoffDir = payload.dir || "";
                    panel.handoffPending = payload.pending || 0;
                } else {
                    panel.whError = error || "The handoff watcher is unreachable.";
                }
            });
    }

    readonly property var payload: bridge && bridge.wallpapers ? bridge.wallpapers : null
    readonly property var items: payload && payload.items ? payload.items : []
    readonly property var current: payload && payload.current ? payload.current : null
    readonly property bool engineOk: payload && payload.engine_ok === true
    readonly property bool loopOn: payload && payload.loop_enabled === true

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "Wallpapers"
        note: panel.items.length > 0 ? panel.items.length + " IMAGES" : ""
    }

    // The engine and the rotation.
    StaleBanner {
        id: staleBanner
        anchors.top: header.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        bridge: panel.bridge
    }

    Card {
        id: controls
        anchors.top: staleBanner.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 46

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight + 2

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "image"
                width: 15
                height: 15
                color: panel.engineOk ? Theme.toneInfo : Theme.toneWarn
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 190
                spacing: 1

                Text {
                    width: parent.width
                    text: panel.payload && panel.payload.engine
                        ? "Engine · " + panel.payload.engine
                        : "Engine · none"
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.bodySmall.size
                    font.weight: Theme.type.bodySmall.weight
                    font.letterSpacing: Theme.type.bodySmall.tracking
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: panel.engineOk
                        ? "ready"
                        : "not answering — the catalog still works"
                    color: panel.engineOk ? Theme.onSurfaceVariant : Theme.toneWarn
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                    elide: Text.ElideRight
                }
            }

            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                icon: "refresh"
                size: 28
                tooltip: "Pick a random wallpaper"
                enabled: panel.bridge
                onClicked: panel.bridge
                    ? panel.bridge.callAndSay("wallpaper.next", {}, "New wallpaper")
                    : null
            }

            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                icon: "repeat"
                size: 28
                active: panel.loopOn
                tooltip: panel.loopOn ? "Stop the rotation" : "Rotate every minute"
                enabled: panel.bridge
                onClicked: panel.bridge
                    ? panel.bridge.callAndSay("wallpaper.loop", {
                        "enabled": !panel.loopOn,
                        "interval_secs": 60
                    }, panel.loopOn ? "Stopped the rotation" : "Rotating every minute")
                    : null
            }
        }
    }

    // The directory.
    Card {
        id: dirCard
        anchors.top: controls.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 36

        Row {
            anchors.fill: parent
            anchors.margins: Theme.spacingTight + 2
            spacing: Theme.spacingTight

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "folder"
                width: 14
                height: 14
                color: Theme.onSurfaceVariant
            }

            TextInput {
                id: dirField
                anchors.verticalCenter: parent.verticalCenter
                width: Math.max(0, parent.width - 110)
                color: Theme.onSurface
                font.family: Theme.monoFamily
                font.pixelSize: Theme.type.labelSmall.size
                font.weight: Theme.type.labelSmall.weight
                font.letterSpacing: Theme.type.labelSmall.tracking
                selectByMouse: true
                selectionColor: Theme.primary
                clip: true
                text: panel.payload && panel.payload.dir ? panel.payload.dir : ""
                // Only the user's own typing writes back. A binding on the
                // text breaks the first time it is typed into, so the field
                // has to be told what to show again when the editing ends.
                onTextEdited: {
                    if (panel.dirEditing)
                        panel.dirDraft = text;
                }
                onAccepted: panel.applyDirectory()
                Keys.onEscapePressed: {
                    panel.dirEditing = false;
                    panel.dirDraft = "";
                    dirField.text = panel.payload && panel.payload.dir
                        ? panel.payload.dir
                        : "";
                    focus = false;
                }
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: !panel.dirEditing
                text: "change"
                color: changeArea.containsMouse ? Theme.onSurface : Theme.onSurfaceVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall.size
                font.weight: Theme.type.labelSmall.weight
                font.letterSpacing: Theme.type.labelSmall.tracking

                MouseArea {
                    id: changeArea
                    anchors.fill: parent
                    anchors.margins: -6
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        panel.dirEditing = true;
                        panel.dirDraft = dirField.text;
                        dirField.forceActiveFocus();
                    }
                }
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                visible: panel.dirEditing
                width: applyLabel.implicitWidth + 16
                height: 24
                radius: Theme.radiusSmall
                color: Theme.primaryDim
                border.width: 1
                border.color: Theme.primary

                Text {
                    id: applyLabel
                    anchors.centerIn: parent
                    text: "Apply"
                    color: Theme.primary
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: panel.applyDirectory()
                }
            }
        }
    }

    function applyDirectory() {
        if (!panel.bridge || !panel.dirEditing)
            return;
        var dir = panel.dirDraft.trim();
        if (dir === "")
            return;
        panel.bridge.callAndSay("wallpaper.setdir", { "dir": dir }, "Watching " + dir);
        panel.dirEditing = false;
        dirField.text = dir;
    }

    // The tabs: Local (the directory above) and Wallhaven (remote).
    Row {
        id: tabs
        anchors.top: dirCard.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 30
        spacing: Theme.spacingTight

        Repeater {
            model: ["local", "wallhaven", "imported"]
            Item {
                required property var modelData
                required property int index
                width: 90
                height: 30

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.radiusSmall
                    color: panel.tab === modelData ? Theme.primaryDim : Theme.surfaceFillLow
                    border.width: 1
                    border.color: panel.tab === modelData ? Theme.primary : Theme.outlineVariant

                    Text {
                        anchors.centerIn: parent
                        text: modelData === "local" ? "Local" : (modelData === "wallhaven" ? "Wallhaven" : "Imported")
                        color: panel.tab === modelData ? Theme.primary : Theme.onSurfaceVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall.size
                        font.weight: Theme.type.labelSmall.weight
                        font.letterSpacing: Theme.type.labelSmall.tracking
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        panel.tab = modelData;
                        if (modelData === "wallhaven" && panel.whItems.length === 0 && !panel.whLoading)
                            panel.whSearch();
                        if (modelData === "imported")
                            panel.handoffStatus();
                    }
                }
            }
        }

        TextField {
            id: whSearchField
            anchors.verticalCenter: parent.verticalCenter
            width: tabs.width - 90 * 3 - Theme.spacingTight * 3
            placeholder: "Search wallhaven…"
            visible: panel.tab === "wallhaven"
            onTextChanged: {
                panel.whQuery = text;
                whTimer.restart();
            }
            onEscapePressed: {
                if (text !== "") text = "";
            }
        }

        TextField {
            id: handoffUrlField
            anchors.verticalCenter: parent.verticalCenter
            width: tabs.width - 90 * 3 - Theme.spacingTight * 3
            placeholder: "Paste a page URL to open…"
            visible: panel.tab === "imported"
            onTextChanged: {
                panel.handoffUrl = text;
            }
            onEscapePressed: {
                if (text !== "") text = "";
            }
            onAccepted: {
                if (panel.bridge && text.trim() !== "") {
                    panel.bridge.callAndSay(
                        "wallpaper.handoff-open",
                        { "url": text.trim() },
                        "Opened — save to Downloads",
                        "Could not open that page");
                }
            }
        }
    }

    // The grid. It has to scroll: a directory of wallpapers is usually bigger
    // than the panel, and a grid that only shows the first eight is a grid
    // with a wall of nothing at the bottom.
    Flickable {
        id: scroller
        anchors.top: tabs.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        contentWidth: width
        contentHeight: {
            if (panel.tab === "imported")
                return 90;
            return (panel.tab === "wallhaven" ? whGrid.height : grid.height) + (panel.tab === "wallhaven" ? 36 : 0);
        }
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        // A scroll indicator, because QtQuick.Controls is not available and
        // there is no other way to say there is more below.
        Rectangle {
            anchors.right: parent.right
            anchors.rightMargin: -Theme.spacingTight
            width: 3
            radius: 1.5
            color: Theme.onSurfaceVariant
            opacity: scroller.moving || scroller.atYBeginning ? 0 : 0.5
            height: Math.max(24, scroller.height * (scroller.height / Math.max(1, scroller.contentHeight)))
            y: scroller.contentY * (scroller.height / Math.max(1, scroller.contentHeight))

            Behavior on opacity {
                NumberAnimation {
                    duration: Theme.crossfadeDuration
                }
            }
        }

        Grid {
            id: grid
            width: scroller.width
            columns: 4
            columnSpacing: Theme.spacingTight
            rowSpacing: Theme.spacingTight
            visible: panel.tab === "local"

        Repeater {
            model: panel.items

            Item {
                required property var modelData

                width: (grid.width - Theme.spacingTight * 3) / 4
                height: width * 0.62

                Rectangle {
                    id: frame
                    anchors.fill: parent
                    radius: Theme.radiusSmall
                    color: Theme.surfaceFillLow
                    border.width: panel.current === modelData.path ? 2 : 0
                    border.color: Theme.primary
                    clip: true

                    Image {
                        anchors.fill: parent
                        anchors.margins: 1
                        // The thumbnail is the file itself, decoded small.
                        // The catalog holds paths, not copies.
                        source: panel.visible && modelData.path ? "file://" + modelData.path : ""
                        sourceSize: Qt.size(240, 150)
                        asynchronous: true
                        fillMode: Image.PreserveAspectCrop
                        smooth: true
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: parent.radius
                        color: "transparent"
                        border.width: 1
                        border.color: Theme.outlineVariant
                    }
                }

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.radiusSmall
                    color: "transparent"
                    visible: tileArea.containsMouse

                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 1
                        radius: Theme.radiusSmall
                        color: Theme.scrim
                    }
                }

                Text {
                    anchors.bottom: parent.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.margins: 5
                    visible: tileArea.containsMouse
                    text: modelData.name
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                    elide: Text.ElideRight
                }

                MouseArea {
                    id: tileArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: panel.bridge
                        ? panel.bridge.callAndSay(
                            "wallpaper.set",
                            { "path": modelData.path },
                            "Wallpaper set")
                        : null
                }
            }
        }
        }

        // The remote Wallhaven grid. Thumbnails are remote URLs, but the
        // QML engine cannot load them directly, so the daemon downloads them
        // alongside search: `thumb_path` in the search payload points at the
        // local cache. The grid paints instantly from disk.
        Grid {
            id: whGrid
            width: scroller.width
            columns: 4
            columnSpacing: Theme.spacingTight
            rowSpacing: Theme.spacingTight
            visible: panel.tab === "wallhaven"

            Repeater {
                model: panel.whItems

                Item {
                    required property var modelData

                    width: (whGrid.width - Theme.spacingTight * 3) / 4
                    height: width * 0.62

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSmall
                        color: Theme.surfaceFillLow
                        clip: true

                        Image {
                            anchors.fill: parent
                            anchors.margins: 1
                            source: panel.visible && modelData.thumb_path ? "file://" + modelData.thumb_path : ""
                            sourceSize: Qt.size(240, 150)
                            asynchronous: true
                            fillMode: Image.PreserveAspectCrop
                            smooth: true
                        }

                        // The free palette, as a sliver at the bottom.
                        Row {
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 4
                            spacing: 0
                            visible: modelData.colors && modelData.colors.length > 0
                            Repeater {
                                model: modelData.colors
                                Rectangle {
                                    required property var modelData
                                    width: whGrid.width / 4 / modelData.length
                                    height: 4
                                    color: modelData
                                }
                            }
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: parent.radius
                            color: "transparent"
                            border.width: 1
                            border.color: Theme.outlineVariant
                        }
                    }

                    Text {
                        anchors.bottom: parent.bottom
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.margins: 5
                        visible: whTile.containsMouse
                        text: modelData.resolution + (modelData.source ? " · " + modelData.source : "")
                        color: Theme.onSurface
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall.size
                        font.weight: Theme.type.labelSmall.weight
                        font.letterSpacing: Theme.type.labelSmall.tracking
                        elide: Text.ElideRight
                    }

                    MouseArea {
                        id: whTile
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.bridge
                            ? panel.bridge.callAndSay(
                                "wallpaper.download",
                                { "url": modelData.path },
                                "Downloaded into your catalog")
                            : null
                    }
                }
            }
        }

        Row {
            anchors.top: (panel.tab === "wallhaven" ? whGrid : grid).bottom
            anchors.topMargin: Theme.spacingTight
            anchors.horizontalCenter: parent.horizontalCenter
            visible: panel.tab === "wallhaven"
            spacing: Theme.spacingTight

            IconButton {
                icon: "chev_left"
                size: 28
                enabled: panel.whPage > 1 && !panel.whLoading
                tooltip: "Older results"
                onClicked: {
                    if (panel.whPage > 1) {
                        panel.whPage -= 1;
                        panel.whSearch();
                    }
                }
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: panel.whLoading ? "Searching…" : ("Page " + panel.whPage + " / " + panel.whLastPage)
                color: Theme.onSurfaceVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall.size
                font.weight: Theme.type.labelSmall.weight
            }

            IconButton {
                icon: "chev_right"
                size: 28
                enabled: panel.whPage < panel.whLastPage && !panel.whLoading
                tooltip: "Newer results"
                onClicked: {
                    if (panel.whPage < panel.whLastPage) {
                        panel.whPage += 1;
                        panel.whSearch();
                    }
                }
            }
        }

        Text {
            anchors.top: (panel.tab === "wallhaven" ? whGrid : grid).bottom
            anchors.topMargin: Theme.spacingTight
            anchors.horizontalCenter: parent.horizontalCenter
            visible: panel.tab === "wallhaven" && panel.whError !== ""
            text: panel.whError
            color: Theme.toneWarn
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
        }

        Column {
            visible: panel.tab === "imported"
            anchors.centerIn: parent
            spacing: Theme.spacingTight
            width: parent.width - Theme.spacing * 2

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: panel.handoffPending > 0
                    ? "Waiting for a save… " + panel.handoffPending + " pending"
                    : "Save a wallpaper to Downloads — the daemon imports it."
                color: Theme.onSurfaceVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.bodySmall.size
                font.weight: Theme.type.bodySmall.weight
                font.letterSpacing: Theme.type.bodySmall.tracking
            }

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: panel.handoffDir ? panel.handoffDir : "Watching ~/Downloads"
                color: Theme.onSurfaceVariant
                font.family: Theme.monoFamily
                font.pixelSize: Theme.type.labelSmall.size
                font.weight: Theme.type.labelSmall.weight
                font.letterSpacing: Theme.type.labelSmall.tracking
            }

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: panel.handoffPending > 0
                    ? panel.handoffPending + " file(s) to import"
                    : "Videos are not imported — the engine cannot draw them."
                color: Theme.onSurfaceVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall.size
                font.weight: Theme.type.labelSmall.weight
                font.letterSpacing: Theme.type.labelSmall.tracking
            }
        }
    }

    Text {
        id: localEmptyText
        anchors.centerIn: scroller
        visible: panel.tab === "local" && panel.items.length === 0
        text: "No images in that directory."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    Text {
        anchors.top: localEmptyText.bottom
        anchors.topMargin: 2
        anchors.horizontalCenter: scroller.horizontalCenter
        width: scroller.width
        visible: panel.tab === "local" && panel.items.length === 0
        text: "Drop images in there, browse Wallhaven, or paste a page URL in Imported."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelSmall.size
        font.weight: Theme.type.labelSmall.weight
        font.letterSpacing: Theme.type.labelSmall.tracking
        horizontalAlignment: Text.AlignHCenter
    }
}