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
    /// The Wallhaven tier: "sfw" (default), "sketchy" or "nsfw". Sketchy
    /// and NSFW need an API key (wallhaven.cc/settings); the daemon
    /// refuses them without one, so the chips persist the tier and the
    /// search sends it.
    property string whPurity: "sfw"
    /// Whether the daemon has a Wallhaven key saved. While true the key
    /// box stays folded into a "Reset key" chip; it unfolds the moment
    /// a higher tier is picked without a key, so the user always sees
    /// where to type it.
    property bool whKeySaved: false
    property bool whKeyEditing: false

    Component.onCompleted: {
        if (panel.bridge && panel.bridge.settings && panel.bridge.settings.wallhaven_purity)
            panel.whPurity = panel.bridge.settings.wallhaven_purity;
        if (panel.bridge && panel.bridge.settings && panel.bridge.settings.wallhaven_api_key)
            panel.whKeySaved = true;
    }
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
            { "q": panel.whQuery, "page": panel.whPage, "purity": panel.whPurity },
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

    // Now showing: the current wallpaper's thumbnail, its name, the engine
    // status, and the rotation controls — one strip instead of a stack of
    // cards. The current image is the answer to "what is my wallpaper?".
    Card {
        id: nowStrip
        anchors.top: staleBanner.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 58

        Row {
            anchors.fill: parent
            anchors.margins: 9
            spacing: 12

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 40
                height: 40
                radius: Theme.radiusSmall
                color: Theme.surfaceFillLow
                border.width: 1
                border.color: Theme.outlineVariant
                clip: true

                Image {
                    anchors.fill: parent
                    anchors.margins: 1
                    visible: panel.current !== null
                    source: panel.visible && panel.current ? "file://" + panel.current : ""
                    sourceSize: Qt.size(160, 160)
                    asynchronous: true
                    fillMode: Image.PreserveAspectCrop
                    smooth: true
                }

                Icon {
                    anchors.centerIn: parent
                    visible: panel.current === null
                    name: "image"
                    width: 18
                    height: 18
                    color: Theme.onSurfaceVariant
                }
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                width: 205
                spacing: 2

                Text {
                    width: parent.width
                    text: "NOW SHOWING"
                    wrapMode: Text.NoWrap
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: panel.current
                        ? panel.current.replace(/^.*\//, "")
                        : "Nothing set"
                    wrapMode: Text.NoWrap
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.titleMedium.size
                    font.weight: Theme.type.titleMedium.weight
                    font.letterSpacing: Theme.type.titleMedium.tracking
                    elide: Text.ElideRight
                }
            }

            Row {
                anchors.verticalCenter: parent.verticalCenter
                width: 118
                spacing: 6

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 8
                    height: 8
                    radius: 4
                    color: panel.engineOk ? Theme.toneInfo : Theme.toneWarn
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - 14
                    text: panel.engineOk
                        ? (panel.payload && panel.payload.engine ? panel.payload.engine : "awww") + " · ready"
                        : "engine down"
                    wrapMode: Text.NoWrap
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

    // The directory, slimmed to a single line — it only needs the path.
    Item {
        id: dirCard
        anchors.top: nowStrip.bottom
        anchors.topMargin: 2
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 32

        Row {
            anchors.fill: parent
            anchors.margins: 2
            spacing: 8

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

    // Appearance is dark-only by decision: the shell is for one user and
    // light mode is not wanted. settings.theme stays in the daemon for the
    // wallpaper palette tie-in, but the picker no longer exposes a switch.

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

    // The tabs: Local (the directory above) and Wallhaven (remote). One
    // Material segmented control holds them; the selected segment is a
    // filled pill inside the shared outline, not three separate cards.
    // The search fields sit to its right.
    Row {
        id: tabs
        anchors.top: dirCard.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 44
        spacing: Theme.spacingTight

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 268
            height: 40
            radius: 20
            color: "transparent"
            border.width: 1
            border.color: Theme.outlineVariant

            Row {
                anchors.fill: parent
                anchors.leftMargin: 2
                anchors.rightMargin: 2
                anchors.topMargin: 2
                anchors.bottomMargin: 2
                spacing: 4

                Repeater {
                    model: ["local", "wallhaven", "imported"]
                    Item {
                        required property var modelData
                        required property int index
                        width: 84
                        height: 36

                        Rectangle {
                            anchors.fill: parent
                            radius: 18
                            color: panel.tab === modelData ? Theme.primaryDim : "transparent"

                            Text {
                                anchors.centerIn: parent
                                text: modelData === "local" ? "Local" : (modelData === "wallhaven" ? "Wallhaven" : "Imported")
                                color: panel.tab === modelData ? Theme.primary : Theme.onSurfaceVariant
                                font.family: Theme.fontFamily
                                font.pixelSize: Theme.type.labelLarge.size
                                font.weight: Theme.type.labelLarge.weight
                                font.letterSpacing: Theme.type.labelLarge.tracking
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
            }
        }

        TextField {
            id: whSearchField
            anchors.verticalCenter: parent.verticalCenter
            width: tabs.width - 268 - Theme.spacingTight
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

    // The Wallhaven tier row: the purity chips plus the API key field.
    // Visible only in the Wallhaven tab; the scroller's top follows it so
    // the other tabs do not inherit its height.
    Row {
        id: tierRow
        anchors.top: tabs.bottom
        anchors.topMargin: Theme.spacingTight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        height: 28
        visible: panel.tab === "wallhaven"
        spacing: Theme.spacingTight

        Repeater {
            model: [
                { "name": "sfw", "label": "SFW" },
                { "name": "sketchy", "label": "Sketchy" },
                { "name": "nsfw", "label": "NSFW" }
            ]
            Item {
                required property var modelData
                width: 72
                height: 28

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.radiusSmall
                    color: panel.whPurity === modelData.name ? Theme.primaryDim : Theme.surfaceFillLow
                    border.width: 1
                    border.color: panel.whPurity === modelData.name ? Theme.primary : Theme.outlineVariant

                    Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: panel.whPurity === modelData.name ? Theme.primary : Theme.onSurfaceVariant
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
                        panel.whPurity = modelData.name;
                        if (!panel.whKeySaved && modelData.name !== "sfw")
                            panel.whKeyEditing = true;
                        if (panel.bridge)
                            panel.bridge.callAndSay(
                                "settings.set",
                                { "key": "wallhaven_purity", "value": modelData.name },
                                "Wallhaven: " + modelData.label,
                                "Could not save the tier");
                        panel.whPage = 1;
                        panel.whSearch();
                    }
                }
            }
        }

        // The key area. Tiny by decision: a short field for the walk-in
        // case, and a "Reset key" pill once a key is saved — the box
        // never sits there unused.
        Item {
            width: tabs.width - 268 - Theme.spacingTight
            height: 28

            Rectangle {
                visible: !panel.whKeySaved || panel.whKeyEditing
                anchors.verticalCenter: parent.verticalCenter
                width: 150
                height: 26
                radius: Theme.radiusSmall
                color: Theme.surfaceFillLow
                border.width: input.activeFocus ? 2 : 1
                border.color: input.activeFocus ? Theme.primary : Theme.outlineVariant

                TextInput {
                    id: input
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.onSurface
                    selectionColor: Theme.primary
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    selectByMouse: true
                    clip: true
                    onAccepted: {
                        if (panel.bridge && text.trim() !== "") {
                            panel.bridge.callAndSay(
                                "settings.set",
                                { "key": "wallhaven_api_key", "value": text.trim() },
                                "Wallhaven API key saved",
                                "Could not save the key");
                            panel.whKeySaved = true;
                        }
                        text = "";
                        panel.whKeyEditing = false;
                        panel.whSearch();
                    }
                    Keys.onEscapePressed: (event) => {
                        if (text !== "") text = "";
                        else if (panel.whKeySaved) {
                            panel.whKeyEditing = false;
                            event.accepted = true;
                        }
                    }
                }

                Text {
                    visible: input.text === ""
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: panel.whKeySaved ? "New key (blank keeps saved)" : "Wallhaven API key"
                    wrapMode: Text.NoWrap
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    elide: Text.ElideRight
                    width: parent.width - 16
                }
            }

            Row {
                visible: panel.whKeySaved && !panel.whKeyEditing
                anchors.verticalCenter: parent.verticalCenter
                spacing: 8

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "check"
                    width: 14
                    height: 14
                    color: Theme.toneGood
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "key saved"
                    wrapMode: Text.NoWrap
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                }

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 88
                    height: 26
                    radius: 13
                    color: Theme.surfaceFillLow
                    border.width: 1
                    border.color: Theme.outlineVariant

                    Text {
                        anchors.centerIn: parent
                        text: "Reset key"
                        wrapMode: Text.NoWrap
                        color: Theme.onSurfaceVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall.size
                        font.weight: Theme.type.labelSmall.weight
                        font.letterSpacing: Theme.type.labelSmall.tracking
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        hoverEnabled: true
                        onClicked: {
                            if (panel.bridge)
                                panel.bridge.callAndSay(
                                    "settings.set",
                                    { "key": "wallhaven_api_key", "value": "" },
                                    "Wallhaven API key removed",
                                    "Could not remove the key");
                            panel.whKeySaved = false;
                            panel.whKeyEditing = true;
                            panel.whSearch();
                        }
                    }
                }
            }
        }
    }

    // The grid. It has to scroll: a directory of wallpapers is usually bigger
    // than the panel, and a grid that only shows the first eight is a grid
    // with a wall of nothing at the bottom.
    Flickable {
        id: scroller
        anchors.top: panel.tab === "wallhaven" ? tierRow.bottom : tabs.bottom
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
            columns: 3
            columnSpacing: Theme.spacingTight
            rowSpacing: Theme.spacingTight
            visible: panel.tab === "local"

        Repeater {
            model: panel.items

            Item {
                required property var modelData

                width: (grid.width - Theme.spacingTight * 2) / 3
                height: width * 0.625

                Rectangle {
                    id: frame
                    anchors.fill: parent
                    radius: Theme.radiusMedium
                    color: Theme.surfaceFillLow
                    clip: true

                    Image {
                        anchors.fill: parent
                        // The thumbnail is the file itself, decoded small.
                        // The catalog holds paths, not copies.
                        source: panel.visible && modelData.path ? "file://" + modelData.path : ""
                        sourceSize: Qt.size(320, 200)
                        asynchronous: true
                        fillMode: Image.PreserveAspectCrop
                        smooth: true
                    }

                    // The glyph shows through while the image is decoding
                    // (and on decode failures), so the tile is never an
                    // empty box.
                    Icon {
                        anchors.centerIn: parent
                        z: -1
                        name: "image"
                        width: 20
                        height: 20
                        color: Theme.onSurfaceVariant
                        opacity: 0.35
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: parent.radius
                        color: "transparent"
                        border.width: panel.current === modelData.path ? 2 : 1
                        border.color: panel.current === modelData.path ? Theme.primary : Theme.outlineVariant
                    }

                    // The active wallpaper announces itself without needing
                    // the hover state.
                    Rectangle {
                        visible: panel.current === modelData.path
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.margins: 6
                        width: nowLabel.implicitWidth + 14
                        height: 18
                        radius: 9
                        color: Theme.primaryDim
                        border.width: 1
                        border.color: Theme.primary

                        Text {
                            id: nowLabel
                            anchors.centerIn: parent
                            text: "Now"
                            color: Theme.primary
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.labelSmall.size
                            font.weight: Theme.type.labelLarge.weight
                            font.letterSpacing: Theme.type.labelSmall.tracking
                        }
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: parent.radius
                        visible: tileArea.containsMouse

                        Rectangle {
                            anchors.fill: parent
                            // A shallow scrim only at the bottom, where the
                            // metadata sits, instead of dulling the image.
                            color: Theme.scrim
                        }
                    }
                }

                Column {
                    anchors.bottom: parent.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.margins: 8
                    visible: tileArea.containsMouse
                    spacing: 0

                    Text {
                        width: parent.width - 44
                        text: modelData.name
                        wrapMode: Text.NoWrap
                        color: Theme.onSurface
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.titleSmall.size
                        font.weight: Theme.type.titleSmall.weight
                        font.letterSpacing: Theme.type.titleSmall.tracking
                        elide: Text.ElideRight
                    }
                }

                Rectangle {
                    anchors.bottom: parent.bottom
                    anchors.right: parent.right
                    anchors.margins: 6
                    width: 34
                    height: 34
                    radius: 17
                    visible: tileArea.containsMouse
                    color: Theme.primary

                    Icon {
                        anchors.centerIn: parent
                        name: "check"
                        width: 18
                        height: 18
                        color: Theme.primaryInk
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.bridge
                            ? panel.bridge.callAndSay(
                                "wallpaper.set",
                                { "path": modelData.path },
                                "Wallpaper set")
                            : null
                    }
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
            columns: 3
            columnSpacing: Theme.spacingTight
            rowSpacing: Theme.spacingTight
            visible: panel.tab === "wallhaven"

            Repeater {
                model: panel.whItems

                Item {
                    required property var modelData

                    width: (whGrid.width - Theme.spacingTight * 2) / 3
                    height: width * 0.625

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusMedium
                        color: Theme.surfaceFillLow
                        clip: true

                        Image {
                            anchors.fill: parent
                            source: panel.visible && modelData.thumb_path ? "file://" + modelData.thumb_path : ""
                            sourceSize: Qt.size(320, 200)
                            asynchronous: true
                            fillMode: Image.PreserveAspectCrop
                            smooth: true
                        }

                        Icon {
                            anchors.centerIn: parent
                            z: -1
                            name: "image"
                            width: 20
                            height: 20
                            color: Theme.onSurfaceVariant
                            opacity: 0.35
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
                                    width: whGrid.width / 3 / modelData.length
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

                        Rectangle {
                            anchors.fill: parent
                            radius: parent.radius
                            visible: whTile.containsMouse

                            Rectangle {
                                anchors.fill: parent
                                color: Theme.scrim
                            }
                        }
                    }

                    Column {
                        anchors.bottom: parent.bottom
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.margins: 8
                        visible: whTile.containsMouse
                        spacing: 0

                        Text {
                            width: parent.width - 44
                            text: modelData.resolution + " · " + Math.round(modelData.file_size / 104857.6) / 10 + " MB"
                            wrapMode: Text.NoWrap
                            color: Theme.onSurface
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.titleSmall.size
                            font.weight: Theme.type.titleSmall.weight
                            font.letterSpacing: Theme.type.titleSmall.tracking
                            elide: Text.ElideRight
                        }
                    }

                    Rectangle {
                        anchors.bottom: parent.bottom
                        anchors.right: parent.right
                        anchors.margins: 6
                        width: 34
                        height: 34
                        radius: 17
                        visible: whTile.containsMouse
                        color: Theme.primary

                        Icon {
                            anchors.centerIn: parent
                            name: "download"
                            width: 18
                            height: 18
                            color: Theme.primaryInk
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: panel.bridge
                                ? panel.bridge.callAndSay(
                                    "wallpaper.download",
                                    { "url": modelData.path, "apply": true },
                                    "Downloaded and set the wallpaper")
                                : null
                        }
                    }

                    MouseArea {
                        id: whTile
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.bridge
                            ? panel.bridge.callAndSay(
                                "wallpaper.download",
                                { "url": modelData.path, "apply": true },
                                "Downloaded and set the wallpaper")
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