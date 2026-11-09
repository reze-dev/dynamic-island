import QtQuick
import ".."

// One activity, in the two shapes the island needs.
//
// Compact is the peek view: a glyph and a single line. Full is the
// triggered view: a glyph, a headline and a supporting line. The colour
// comes from the activity's tone, which is what makes the island's dot
// change colour with whatever woke it.
Item {
    id: root

    property var activity: null
    property bool compact: false
    property bool showGlyph: true

    readonly property color tone: Theme.toneColor(activity && activity.tone ? activity.tone : "neutral")
    readonly property string title: activity && activity.title ? activity.title : ""
    readonly property string subtitle: activity && activity.subtitle ? activity.subtitle : ""
    readonly property string glyph: activity && activity.glyph ? activity.glyph : "spark"

    implicitHeight: compact ? 24 : 40

    Rectangle {
        id: badge
        visible: root.showGlyph
        width: root.compact ? 24 : 30
        height: width
        radius: width / 2
        color: Theme.stateLayer(root.tone, 0.16)
        border.width: 1
        border.color: Theme.stateLayer(root.tone, 0.28)

        Icon {
            anchors.centerIn: parent
            name: root.glyph
            width: Math.round(parent.width * 0.55)
            height: width
            color: root.tone
        }
    }

    Column {
        anchors.left: badge.visible ? badge.right : parent.left
        anchors.leftMargin: badge.visible ? Theme.spacingTight + 4 : 0
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        // The column shares the pill's whole height (40): two lines at
        // bodyLarge + bodySmall are a full cell taller than the band, so
        // their edges clip. One step smaller keeps both rows inside.
        spacing: 0

        SlideText {
            width: parent.width
            text: root.title
            color: Theme.onSurface
            pixelSize: Theme.type.bodyMedium.size
            weight: Theme.type.bodyMedium.weight
            tracking: Theme.type.bodyMedium.tracking
        }

        SlideText {
            width: parent.width
            visible: !root.compact && root.subtitle !== ""
            text: root.subtitle
            color: Theme.onSurfaceVariant
            pixelSize: Theme.type.bodySmall.size
            weight: Theme.type.bodySmall.weight
        }
    }
}
