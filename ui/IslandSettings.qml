pragma Singleton

import QtQuick

QtObject {
    property int morphApproach: 0
    readonly property bool keyboardFocus: true

    readonly property string approachName: morphApproach === 0 ? "A: surface resizes" : "B: fixed surface"
}