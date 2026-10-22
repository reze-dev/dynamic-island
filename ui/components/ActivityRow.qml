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
        width: root.compact ? 24 : 32
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
        spacing: 1

        Text {
            width: parent.width
            text: root.title
            color: Theme.onSurface
            font.family: Theme.fontFamily
            font.pixelSize: root.compact ? Theme.type.bodyMedium.size : Theme.type.bodyLarge.size
            font.weight: root.compact ? Theme.type.bodyMedium.weight : Theme.type.bodyLarge.weight
            font.letterSpacing: root.compact ? Theme.type.bodyMedium.tracking : Theme.type.bodyLarge.tracking
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            visible: !root.compact && root.subtitle !== ""
            text: root.subtitle
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodySmall.size
            font.letterSpacing: Theme.type.bodySmall.tracking
            elide: Text.ElideRight
        }
    }
}
